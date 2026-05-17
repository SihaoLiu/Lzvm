use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::key_directory::{read_key_directory_catalog, KeyDirectoryCatalog};
use lzvm_prover::setup_preflight::validate_setup_directory_manifest_if_present;
use lzvm_prover::{
    derive_prove_run_plan, GpuRunOptions, ProveExecutionPlan, ProvePartitionPlan, ProvePassKind,
    ProvePassRequest, ProveRunOptions, ProveRunPlan, ProveRunRequest,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove plan failed: {message}");
            return 1;
        }
    };

    let catalog = match read_checked_setup_catalog(&parsed.setup_dir) {
        Ok(catalog) => catalog,
        Err(message) => {
            let _ = writeln!(stderr, "prove plan failed: {message}");
            return 1;
        }
    };

    let plan = match derive_prove_run_plan(&catalog, parsed.request) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove plan failed: {error}");
            return 1;
        }
    };

    write_run_plan_summary(stdout, &plan);
    0
}

struct ParsedProvePlan {
    setup_dir: PathBuf,
    request: ProveRunRequest,
}

#[derive(Debug)]
pub(crate) struct ParsedRunArgs {
    pub positionals: Vec<PathBuf>,
    pub request: ProveRunRequest,
    pub program_image_cache: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PassSelection {
    Full,
    Contributions,
    Internal { contribution_count: usize },
}

pub(crate) fn read_checked_setup_catalog(path: &Path) -> Result<KeyDirectoryCatalog, String> {
    let catalog = read_key_directory_catalog(path).map_err(|error| error.to_string())?;
    validate_setup_directory_manifest_if_present(path, &catalog)
        .map_err(|error| error.to_string())?;
    Ok(catalog)
}

#[derive(Debug)]
pub(crate) enum ParseError {
    Usage,
    Invalid(String),
}

fn parse_args(args: &[&str]) -> Result<ParsedProvePlan, ParseError> {
    let parsed = parse_run_args(args, 2, 2)?;
    if parsed.program_image_cache.is_some() {
        return Err(ParseError::Invalid(
            "--program-image-cache is not supported for prove plan".to_owned(),
        ));
    }
    Ok(ParsedProvePlan {
        setup_dir: parsed.positionals[0].clone(),
        request: parsed.request,
    })
}

pub(crate) fn parse_run_args(
    args: &[&str],
    min_positionals: usize,
    max_positionals: usize,
) -> Result<ParsedRunArgs, ParseError> {
    let mut aggregate = false;
    let mut remote_aggregation = false;
    let mut final_wrap = false;
    let mut verify_outputs = true;
    let mut save_outputs = false;
    let mut minimal_memory = false;
    let mut gpu = GpuRunOptions::default();
    let mut gpu_streams_used = false;
    let mut input_data = None;
    let mut program_image_cache = None;
    let mut pass_selection = None;
    let mut partition_option_used = false;
    let mut partition_count = 1_usize;
    let mut partition_ids = vec![0_u32];
    let mut worker_index = 0_usize;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "--aggregate" => aggregate = true,
            "--remote-aggregation" => remote_aggregation = true,
            "--final-wrap" => final_wrap = true,
            "--no-verify-outputs" => verify_outputs = false,
            "--save-outputs" => save_outputs = true,
            "--minimal-memory" => minimal_memory = true,
            "--gpu-preallocate" => gpu.preallocate = true,
            "--no-pack-trace" => gpu.pack_trace = false,
            "--contributions" => {
                set_pass_selection(&mut pass_selection, PassSelection::Contributions)?;
            }
            "--internal-contributions" => {
                index += 1;
                let contribution_count = parse_usize(args.get(index), "--internal-contributions")?;
                set_pass_selection(
                    &mut pass_selection,
                    PassSelection::Internal { contribution_count },
                )?;
            }
            "--gpu-streams" => {
                index += 1;
                if std::mem::replace(&mut gpu_streams_used, true) {
                    return Err(ParseError::Invalid(
                        "duplicate --gpu-streams option".to_owned(),
                    ));
                }
                gpu.max_streams = parse_usize(args.get(index), "--gpu-streams")?;
            }
            "--witness-thread-pools" => {
                index += 1;
                gpu.witness_thread_pools = parse_usize(args.get(index), "--witness-thread-pools")?;
            }
            "--stored-witnesses" => {
                index += 1;
                gpu.max_stored_witnesses = parse_usize(args.get(index), "--stored-witnesses")?;
            }
            "--partitions" => {
                index += 1;
                partition_option_used = true;
                partition_count = parse_usize(args.get(index), "--partitions")?;
            }
            "--partition-ids" => {
                index += 1;
                partition_option_used = true;
                partition_ids = parse_partition_ids(args.get(index))?;
            }
            "--worker" => {
                index += 1;
                partition_option_used = true;
                worker_index = parse_usize(args.get(index), "--worker")?;
            }
            "--input-data" => {
                index += 1;
                partition_option_used = true;
                let value = args
                    .get(index)
                    .ok_or(ParseError::Invalid("missing --input-data value".to_owned()))?;
                if input_data.replace(PathBuf::from(*value)).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --input-data option".to_owned(),
                    ));
                }
            }
            "--program-image-cache" => {
                index += 1;
                let value = args.get(index).ok_or(ParseError::Invalid(
                    "missing --program-image-cache value".to_owned(),
                ))?;
                if program_image_cache.replace(PathBuf::from(*value)).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --program-image-cache option".to_owned(),
                    ));
                }
            }
            value if value.starts_with("--") => {
                return Err(ParseError::Invalid(format!("unknown option {value}")));
            }
            value => positionals.push(value),
        }
        index += 1;
    }

    if positionals.len() < min_positionals || positionals.len() > max_positionals {
        return Err(ParseError::Usage);
    }

    let positionals = positionals
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let output_dir = positionals[1].clone();
    let options = ProveRunOptions {
        aggregate,
        remote_aggregation,
        final_wrap,
        verify_outputs,
        save_outputs,
        minimal_memory,
        output_dir,
    };
    let pass = match pass_selection.unwrap_or(PassSelection::Full) {
        PassSelection::Full => ProvePassRequest::Full(ProvePartitionPlan {
            input_data,
            partition_count,
            partition_ids,
            worker_index,
        }),
        PassSelection::Contributions => ProvePassRequest::Contributions(ProvePartitionPlan {
            input_data,
            partition_count,
            partition_ids,
            worker_index,
        }),
        PassSelection::Internal { contribution_count } => {
            if partition_option_used {
                return Err(ParseError::Invalid(
                    "partition options require a partitioned prove pass".to_owned(),
                ));
            }
            ProvePassRequest::Internal { contribution_count }
        }
    };

    Ok(ParsedRunArgs {
        positionals,
        request: ProveRunRequest { pass, options, gpu },
        program_image_cache,
    })
}

fn set_pass_selection(
    current: &mut Option<PassSelection>,
    selected: PassSelection,
) -> Result<(), ParseError> {
    if current.replace(selected).is_some() {
        return Err(ParseError::Invalid(
            "multiple prove pass options are not supported".to_owned(),
        ));
    }
    Ok(())
}

fn parse_usize(value: Option<&&str>, option: &str) -> Result<usize, ParseError> {
    value
        .ok_or_else(|| ParseError::Invalid(format!("missing {option} value")))?
        .parse::<usize>()
        .map_err(|_| ParseError::Invalid(format!("{option} value must be an unsigned integer")))
}

fn parse_partition_ids(value: Option<&&str>) -> Result<Vec<u32>, ParseError> {
    let value =
        value.ok_or_else(|| ParseError::Invalid("missing --partition-ids value".to_owned()))?;
    if value.is_empty() {
        return Err(ParseError::Invalid(
            "--partition-ids value must not be empty".to_owned(),
        ));
    }
    value
        .split(',')
        .map(|entry| {
            entry.parse::<u32>().map_err(|_| {
                ParseError::Invalid("--partition-ids entries must be unsigned integers".to_owned())
            })
        })
        .collect()
}

fn format_pass_kind(kind: ProvePassKind) -> &'static str {
    match kind {
        ProvePassKind::Contributions => "contributions",
        ProvePassKind::Internal => "internal",
        ProvePassKind::Full => "full",
    }
}

fn format_partition_ids(partition_ids: &[u32]) -> String {
    partition_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn write_run_plan_summary(stdout: &mut dyn Write, plan: &ProveRunPlan) {
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "pass={}", format_pass_kind(plan.pass.kind()));
    let _ = writeln!(stdout, "units={}", plan.schedule.unit_count);
    let _ = writeln!(stdout, "fixed_bytes={}", plan.schedule.total_fixed_bytes);
    let _ = writeln!(
        stdout,
        "pcs_material_units={}",
        plan.schedule.pcs_material_unit_count
    );
    let _ = writeln!(
        stdout,
        "pcs_material_bytes={}",
        plan.schedule.total_pcs_material_bytes
    );
    let _ = writeln!(stdout, "queries={}", plan.schedule.total_query_count);
    let _ = writeln!(
        stdout,
        "max_extended_domain_bits={}",
        plan.schedule.max_extended_domain_bits
    );
    if let ProvePassRequest::Contributions(partitions) | ProvePassRequest::Full(partitions) =
        &plan.pass
    {
        let _ = writeln!(stdout, "partitions={}", partitions.partition_count);
        let _ = writeln!(
            stdout,
            "partition_ids={}",
            format_partition_ids(&partitions.partition_ids)
        );
        let _ = writeln!(stdout, "worker={}", partitions.worker_index);
        let _ = writeln!(
            stdout,
            "input_data={}",
            partitions
                .input_data
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned())
        );
    } else if let ProvePassRequest::Internal { contribution_count } = &plan.pass {
        let _ = writeln!(stdout, "contribution_count={contribution_count}");
    }
    let _ = writeln!(stdout, "aggregate={}", plan.options.aggregate);
    let _ = writeln!(
        stdout,
        "remote_aggregation={}",
        plan.options.remote_aggregation
    );
    let _ = writeln!(stdout, "final_wrap={}", plan.options.final_wrap);
    let _ = writeln!(stdout, "verify_outputs={}", plan.options.verify_outputs);
    let _ = writeln!(stdout, "save_outputs={}", plan.options.save_outputs);
    let _ = writeln!(stdout, "minimal_memory={}", plan.options.minimal_memory);
    let _ = writeln!(stdout, "output={}", plan.options.output_dir.display());
    let _ = writeln!(stdout, "gpu_preallocate={}", plan.gpu.preallocate);
    let _ = writeln!(stdout, "gpu_streams={}", plan.gpu.max_streams);
    let _ = writeln!(
        stdout,
        "witness_thread_pools={}",
        plan.gpu.witness_thread_pools
    );
    let _ = writeln!(stdout, "stored_witnesses={}", plan.gpu.max_stored_witnesses);
    let _ = writeln!(stdout, "pack_trace={}", plan.gpu.pack_trace);
    let _ = writeln!(
        stdout,
        "setup_hash={}",
        format_hash(&plan.schedule.setup_hash)
    );
}

pub(crate) fn prepare_requested_gpu_setup(
    plan: &ProveExecutionPlan,
) -> Result<(), lzvm_prover::GpuSetupError> {
    if plan.run_plan.gpu.preallocate {
        lzvm_prover::prepare_gpu_setup(plan.run_plan.schedule.max_extended_domain_bits as usize)?;
    }
    Ok(())
}

pub(crate) fn validate_all_unit_stored_witness_limit(
    limit: usize,
    required: usize,
) -> Result<(), String> {
    if limit < required {
        return Err(format!(
            "stored witness limit {limit} is lower than required all-unit witness outputs {required}"
        ));
    }
    Ok(())
}

pub(crate) fn format_hash(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove plan [options] <setup-dir> <output-dir>\n  --contributions\n  --internal-contributions <count>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_prover::ProveSchedule;

    #[test]
    fn parses_program_image_cache_option_for_run_args() {
        let parsed = parse_run_args(
            &[
                "--program-image-cache",
                "cache.bin",
                "setup-dir",
                "out-dir",
                "witness.so",
                "guest.elf",
            ],
            4,
            5,
        )
        .expect("run args should parse");

        assert_eq!(parsed.program_image_cache, Some(PathBuf::from("cache.bin")));
        assert_eq!(parsed.positionals[0], PathBuf::from("setup-dir"));
        assert_eq!(parsed.positionals[3], PathBuf::from("guest.elf"));
    }

    #[test]
    fn rejects_missing_program_image_cache_option_value() {
        let result = parse_run_args(&["--program-image-cache"], 4, 5);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "missing --program-image-cache value"
        ));
    }

    #[test]
    fn rejects_program_image_cache_option_for_plan_args() {
        let result = parse_args(&["--program-image-cache", "cache.bin", "setup-dir", "out-dir"]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "--program-image-cache is not supported for prove plan"
        ));
    }

    #[test]
    fn parses_contribution_pass_for_run_args() {
        let parsed = parse_run_args(
            &[
                "--contributions",
                "--partitions",
                "4",
                "--partition-ids",
                "1,3",
                "--worker",
                "2",
                "--input-data",
                "input.bin",
                "setup-dir",
                "out-dir",
            ],
            2,
            2,
        )
        .expect("run args should parse");

        match parsed.request.pass {
            ProvePassRequest::Contributions(partitions) => {
                assert_eq!(partitions.input_data, Some(PathBuf::from("input.bin")));
                assert_eq!(partitions.partition_count, 4);
                assert_eq!(partitions.partition_ids, vec![1, 3]);
                assert_eq!(partitions.worker_index, 2);
            }
            _ => panic!("expected contributions pass"),
        }
    }

    #[test]
    fn rejects_duplicate_input_data_option() {
        let result = parse_run_args(
            &[
                "--input-data",
                "first.bin",
                "--input-data",
                "second.bin",
                "setup-dir",
                "out-dir",
            ],
            2,
            2,
        );

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message)) if message == "duplicate --input-data option"
        ));
    }

    #[test]
    fn rejects_duplicate_gpu_streams_option() {
        let result = parse_run_args(
            &[
                "--gpu-streams",
                "4",
                "--gpu-streams",
                "8",
                "setup-dir",
                "out-dir",
            ],
            2,
            2,
        );

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message)) if message == "duplicate --gpu-streams option"
        ));
    }

    #[test]
    fn parses_internal_pass_for_run_args() {
        let parsed = parse_run_args(
            &["--internal-contributions", "3", "setup-dir", "out-dir"],
            2,
            2,
        )
        .expect("run args should parse");

        assert!(matches!(
            parsed.request.pass,
            ProvePassRequest::Internal {
                contribution_count: 3
            }
        ));
    }

    #[test]
    fn rejects_partition_options_for_internal_pass() {
        let result = parse_run_args(
            &[
                "--internal-contributions",
                "3",
                "--input-data",
                "input.bin",
                "setup-dir",
                "out-dir",
            ],
            2,
            2,
        );

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "partition options require a partitioned prove pass"
        ));
    }

    #[test]
    fn rejects_multiple_pass_options() {
        let result = parse_run_args(
            &[
                "--contributions",
                "--internal-contributions",
                "3",
                "setup-dir",
                "out-dir",
            ],
            2,
            2,
        );

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "multiple prove pass options are not supported"
        ));
    }

    #[test]
    fn writes_internal_run_plan_summary() {
        let plan = ProveRunPlan {
            schedule: ProveSchedule {
                setup_hash: [7; 32],
                unit_count: 2,
                total_fixed_bytes: 16,
                total_pcs_material_bytes: 32,
                pcs_material_unit_count: 2,
                total_query_count: 9,
                max_extended_domain_bits: 12,
                units: Vec::new(),
            },
            pass: ProvePassRequest::Internal {
                contribution_count: 3,
            },
            options: ProveRunOptions::default_for_output(PathBuf::from("out-dir")),
            gpu: GpuRunOptions::default(),
        };
        let mut stdout = Vec::new();

        write_run_plan_summary(&mut stdout, &plan);
        let text = String::from_utf8(stdout).expect("summary should be utf-8");

        assert!(text.contains("pass=internal\n"));
        assert!(text.contains("contribution_count=3\n"));
        assert!(!text.contains("partitions="));
    }
}
