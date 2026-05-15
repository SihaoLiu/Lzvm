use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::expression_info::{
    encode_expression_info, read_expression_info_binary_file, ExpressionInfo,
};
use lzvm_artifacts::fixed::{
    read_fixed_columns_file, read_fixed_columns_file_for_setup, FixedColumns,
};
use lzvm_artifacts::global_info::{encode_global_info, GlobalInfo};
use lzvm_artifacts::hint_program::{
    encode_regular_hint_program, regular_hint_program_from_expression_info,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, validate_key_directory_layout, KeyDirectoryCatalog,
};
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, encode_pcs_setup_material};
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, read_pcs_setup_plan_file,
};
use lzvm_artifacts::proof::{read_proof_artifact_file, ProofArtifact};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, read_verifier_info_binary_file, VerifierInfo,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::constant_opening::validate_constant_opening_segments;
use lzvm_prover::global_constraints::{
    validate_global_constraints as validate_global_constraint_program, GlobalConstraintInputs,
    GlobalConstraintValidationError,
};
use lzvm_prover::group_values::load_group_values_from_segments;
use lzvm_prover::hint_eval::{global_hint_input_requirements, resolve_global_hint_program};
use lzvm_prover::pcs_fri::{
    load_pcs_fri_opening_segment_from_segments, validate_pcs_fri_opening_folds_from_units,
    validate_pcs_fri_opening_segments,
};
use lzvm_prover::pcs_material_manifest::validate_pcs_material_manifest_segments;
use lzvm_prover::pcs_query_plan::{
    load_pcs_query_plan_from_segments, validate_seeded_pcs_query_plan_segments,
    validate_transcript_pcs_query_plan_segments,
};
use lzvm_prover::pcs_transcript_segments::{
    derive_pcs_transcript_challenges_from_proof_segments,
    derive_pcs_transcript_unit_challenges_from_proof_segments,
};
use lzvm_prover::proof_preflight::validate_proof_public_values;
use lzvm_prover::proof_values::{flatten_pcs_proof_values, load_pcs_proof_values_from_segments};
use lzvm_prover::verifier_query::{
    validate_verifier_query_outputs_from_segments, VerifierFriQueryOutputSegmentsRequest,
};
use lzvm_prover::witness_commitment::load_witness_commitment_segments;
use lzvm_prover::witness_opening::validate_witness_opening_segments;
use lzvm_prover::{derive_prove_schedule, ProveSchedule};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns_with_backend, write_base_constant_tree,
    write_base_fixed_columns, write_constant_tree_leaves_with_backend,
    write_verification_key_from_constant_tree, FixedExtensionBackend,
};

mod prove_inputs;
mod prove_plan;
mod prove_witness;

const GLOBAL_INFO_BINARY_FILE_NAME: &str = "pilout.globalInfo.bin";

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["prove", "inputs", rest @ ..] => prove_inputs::run(rest, stdout, stderr),
        ["prove", "plan", rest @ ..] => prove_plan::run(rest, stdout, stderr),
        ["prove", "schedule", setup_dir] => prove_schedule(setup_dir, stdout, stderr),
        ["prove", "schedule", ..] => write_prove_schedule_usage(stderr),
        ["prove", "witness", rest @ ..] => prove_witness::run(rest, stdout, stderr),
        ["verify", "setup-preflight", setup_dir, proof_bin, public_values_path] => {
            verify_setup_preflight(setup_dir, proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "setup-preflight", ..] => write_verify_setup_preflight_usage(stderr),
        ["verify", "preflight", proof_bin, public_values_path] => {
            verify_preflight(proof_bin, public_values_path, stdout, stderr)
        }
        ["verify", "preflight", ..] => write_verify_preflight_usage(stderr),
        ["setup", "fingerprint", setup_dir] => {
            fingerprint_setup_directory(setup_dir, stdout, stderr)
        }
        ["setup", "fingerprint", ..] => write_fingerprint_usage(stderr),
        ["setup", "validate", setup_dir] => validate_setup_directory(setup_dir, stdout, stderr),
        ["setup", "validate", ..] => write_validate_usage(stderr),
        ["setup", "write-pcs-plan", setup_info_bin, out_pcs_plan] => {
            write_pcs_setup_plan(setup_info_bin, out_pcs_plan, stdout, stderr)
        }
        ["setup", "write-pcs-plan", ..] => write_pcs_plan_usage(stderr),
        ["setup", "write-pcs-material", setup_info_bin, pcs_plan, fixed_const, consttree, out_pcs_material] => {
            write_pcs_setup_material(
                setup_info_bin,
                pcs_plan,
                fixed_const,
                consttree,
                out_pcs_material,
                stdout,
                stderr,
            )
        }
        ["setup", "write-pcs-material", ..] => write_pcs_material_usage(stderr),
        ["setup", "write-fixed-native", setup_info_bin, columns_bin, out_const] => {
            write_fixed_columns_native(setup_info_bin, columns_bin, out_const, stdout, stderr)
        }
        ["setup", "write-fixed-native", ..] => write_fixed_native_usage(stderr),
        ["setup", "write-base-native", "--backend", backend, setup_info_bin, columns_bin, out_const, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base write", stderr)
            else {
                return 1;
            };
            write_base_native(
                setup_info_bin,
                columns_bin,
                out_const,
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-base-native", setup_info_bin, columns_bin, out_const, out_consttree] => {
            write_base_native(
                setup_info_bin,
                columns_bin,
                out_const,
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-base-native", ..] => write_base_native_usage(stderr),
        ["setup", "write-base-directory", "--derive-verkey", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--backend", backend, "--derive-verkey", setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--derive-verkey", setup_dir] => {
            write_base_directory(setup_dir, FixedExtensionBackend::Cpu, true, stdout, stderr)
        }
        ["setup", "write-base-directory", "--backend", backend, setup_dir] => {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native base directory write", stderr)
            else {
                return 1;
            };
            write_base_directory(setup_dir, backend, false, stdout, stderr)
        }
        ["setup", "write-base-directory", setup_dir] => {
            write_base_directory(setup_dir, FixedExtensionBackend::Cpu, false, stdout, stderr)
        }
        ["setup", "write-base-directory", ..] => write_base_directory_usage(stderr),
        ["setup", "write-pcs-directory", setup_dir] => {
            write_pcs_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-directory", ..] => write_pcs_directory_usage(stderr),
        ["setup", "write-pcs-material-directory", setup_dir] => {
            write_pcs_material_directory(setup_dir, stdout, stderr)
        }
        ["setup", "write-pcs-material-directory", ..] => write_pcs_material_directory_usage(stderr),
        ["setup", "write-verkey-native", setup_info_bin, consttree, out_verkey_bin] => {
            write_verification_key_native(setup_info_bin, consttree, out_verkey_bin, stdout, stderr)
        }
        ["setup", "write-verkey-native", ..] => write_verkey_native_usage(stderr),
        ["setup", "write-const-tree", setup_info_bin, tree_bin, root_bin, out_consttree] => {
            write_constant_tree(
                setup_info_bin,
                tree_bin,
                root_bin,
                out_consttree,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-tree", ..] => write_const_tree_usage(stderr),
        ["setup", "write-const-leaves", "--backend", backend, setup_info_bin, columns_bin, out_leaves] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup constant-tree leaf write", stderr)
            else {
                return 1;
            };
            write_constant_tree_leaves_command(
                setup_info_bin,
                columns_bin,
                out_leaves,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-leaves", setup_info_bin, columns_bin, out_leaves] => {
            write_constant_tree_leaves_command(
                setup_info_bin,
                columns_bin,
                out_leaves,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-leaves", ..] => write_const_leaves_usage(stderr),
        ["setup", "write-const-native", "--backend", backend, setup_info_bin, columns_bin, root_bin, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native constant-tree write", stderr)
            else {
                return 1;
            };
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                Some(root_bin),
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", "--backend", backend, setup_info_bin, columns_bin, out_consttree] =>
        {
            let Some(backend) =
                parse_fixed_extension_backend(backend, "setup native constant-tree write", stderr)
            else {
                return 1;
            };
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                None,
                out_consttree,
                backend,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", setup_info_bin, columns_bin, root_bin, out_consttree] => {
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                Some(root_bin),
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", setup_info_bin, columns_bin, out_consttree] => {
            write_constant_tree_native(
                setup_info_bin,
                columns_bin,
                None,
                out_consttree,
                FixedExtensionBackend::Cpu,
                stdout,
                stderr,
            )
        }
        ["setup", "write-const-native", ..] => write_const_native_usage(stderr),
        _ => write_validate_usage(stderr),
    }
}

fn prove_schedule(setup_dir: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let catalog = match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove schedule failed: {error}");
            return 1;
        }
    };
    let schedule = match derive_prove_schedule(&catalog) {
        Ok(schedule) => schedule,
        Err(error) => {
            let _ = writeln!(stderr, "prove schedule failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", schedule.unit_count);
    let _ = writeln!(stdout, "fixed_bytes={}", schedule.total_fixed_bytes);
    let _ = writeln!(
        stdout,
        "pcs_material_units={}",
        schedule.pcs_material_unit_count
    );
    let _ = writeln!(
        stdout,
        "pcs_material_bytes={}",
        schedule.total_pcs_material_bytes
    );
    let _ = writeln!(stdout, "queries={}", schedule.total_query_count);
    let _ = writeln!(
        stdout,
        "max_extended_domain_bits={}",
        schedule.max_extended_domain_bits
    );
    let _ = writeln!(stdout, "setup_hash={}", format_hash(&schedule.setup_hash));
    0
}

fn fingerprint_setup_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => match key_directory_catalog_digest_hex(&catalog) {
            Ok(fingerprint) => {
                let _ = writeln!(stdout, "status=ok");
                let _ = writeln!(stdout, "units={}", catalog.units.len());
                let _ = writeln!(stdout, "fingerprint={fingerprint}");
                0
            }
            Err(error) => {
                let _ = writeln!(stderr, "setup fingerprint failed: {error}");
                1
            }
        },
        Err(error) => {
            let _ = writeln!(stderr, "setup fingerprint failed: {error}");
            1
        }
    }
}

fn verify_preflight(
    proof_bin: &str,
    public_values_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let proof = match read_proof_artifact_file(proof_bin) {
        Ok(proof) => proof,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    let public_values = match read_public_values_file(public_values_path) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };
    let report = match validate_proof_public_values(&proof, &public_values) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify preflight failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "segments={}", report.segment_count);
    let _ = writeln!(stdout, "public_values={}", report.public_value_count);
    0
}

fn verify_setup_preflight(
    setup_dir: &str,
    proof_bin: &str,
    public_values_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let catalog = match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let setup_hash = match key_directory_catalog_digest(&catalog) {
        Ok(setup_hash) => setup_hash,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let proof = match read_proof_artifact_file(proof_bin) {
        Ok(proof) => proof,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let public_values = match read_public_values_file(public_values_path) {
        Ok(public_values) => public_values,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    if proof.setup_hash != public_values.setup_hash {
        let _ = writeln!(stderr, "verify setup-preflight failed: setup hash mismatch");
        return 1;
    }
    if proof.setup_hash != setup_hash {
        let _ = writeln!(
            stderr,
            "verify setup-preflight failed: setup catalog fingerprint mismatch"
        );
        return 1;
    }
    let public_report = match validate_proof_public_values(&proof, &public_values) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    let schedule = match derive_prove_schedule(&catalog) {
        Ok(schedule) => schedule,
        Err(error) => {
            let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
            return 1;
        }
    };
    if let Err(error) = validate_pcs_material_manifest_segments(&schedule, &proof.segments) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_witness_commitment_segments(&schedule, &proof) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_pcs_query_plan(&schedule, &proof, &public_values) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_constant_opening_segment(&schedule, &proof) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_witness_opening_segment(&schedule, &proof) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_global_constraints(&catalog, &schedule, &proof, &public_values) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) = validate_global_hints(&catalog, &schedule, &proof, &public_values) {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }
    if let Err(error) =
        validate_optional_pcs_fri_opening_segment(&catalog, &schedule, &proof, &public_values)
    {
        let _ = writeln!(stderr, "verify setup-preflight failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", catalog.units.len());
    let _ = writeln!(stdout, "segments={}", public_report.segment_count);
    let _ = writeln!(stdout, "public_values={}", public_report.public_value_count);
    0
}

fn validate_witness_commitment_segments(
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
) -> Result<(), String> {
    load_witness_commitment_segments(&schedule.units, &proof.segments)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn validate_pcs_query_plan(
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
    public_values: &lzvm_artifacts::public_values::PublicValues,
) -> Result<(), String> {
    if !uses_transcript_query_plan_inputs(proof) {
        return validate_seeded_pcs_query_plan_segments(
            schedule,
            proof.public_values_hash,
            &proof.segments,
        )
        .map_err(|error| error.to_string());
    }

    let public_value_fields = transcript_public_value_fields(public_values)?;
    validate_transcript_pcs_query_plan_segments(schedule, &public_value_fields, &proof.segments)
        .map_err(|error| error.to_string())
}

fn uses_transcript_query_plan_inputs(proof: &ProofArtifact) -> bool {
    proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_QUERY_NONCE_SEGMENT_ID)
        || proof
            .segments
            .iter()
            .any(|segment| segment.id == PCS_EVALUATION_SEGMENT_ID)
}

fn transcript_public_value_fields(
    public_values: &lzvm_artifacts::public_values::PublicValues,
) -> Result<Vec<Felt>, String> {
    public_values
        .values
        .iter()
        .flat_map(|entry| entry.elements.iter().copied())
        .map(Felt::from_canonical)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("invalid PCS transcript public value: {error}"))
}

fn validate_witness_opening_segment(
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
) -> Result<(), String> {
    validate_witness_opening_segments(&schedule.units, &proof.segments)
        .map_err(|error| error.to_string())
}

fn validate_constant_opening_segment(
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
) -> Result<(), String> {
    validate_constant_opening_segments(&schedule.units, &proof.segments)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_optional_pcs_fri_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<(), String> {
    if !proof
        .segments
        .iter()
        .any(|segment| segment.id == PCS_FRI_OPENING_SEGMENT_ID)
    {
        return Ok(());
    }
    validate_pcs_fri_opening_segments(&schedule.units, &proof.segments)
        .map_err(|error| error.to_string())?;
    if !uses_transcript_query_plan_inputs(proof) {
        return Ok(());
    }

    let query_plan =
        load_pcs_query_plan_from_segments(&proof.segments).map_err(|error| error.to_string())?;
    let opening = load_pcs_fri_opening_segment_from_segments(&proof.segments)
        .map_err(|error| error.to_string())?;
    let public_value_fields = transcript_public_value_fields(public_values)?;
    let transcript_challenges = derive_pcs_transcript_unit_challenges_from_proof_segments(
        schedule,
        &public_value_fields,
        &proof.segments,
    )
    .map_err(|error| error.to_string())?;
    validate_pcs_fri_opening_folds_from_units(
        &schedule.units,
        &query_plan.units,
        &opening.units,
        &transcript_challenges,
    )
    .map_err(|error| error.to_string())?;
    let verifier_codes = catalog
        .units
        .iter()
        .map(|unit| &unit.metadata.verifier.query)
        .collect::<Vec<_>>();
    validate_verifier_query_outputs_from_segments(VerifierFriQueryOutputSegmentsRequest {
        units: &schedule.units,
        verifier_codes: &verifier_codes,
        global_info: &catalog.layout.global_info,
        public_values: &public_value_fields,
        query_units: &query_plan.units,
        opening_units: &opening.units,
        transcript_challenges: &transcript_challenges,
        segments: &proof.segments,
    })
    .map_err(|error| error.to_string())
}

fn validate_global_constraints(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<(), String> {
    if catalog.global_constraints.entries.is_empty() {
        return Ok(());
    }

    let publics = transcript_public_value_fields(public_values)?;
    let proof_values = load_pcs_proof_values(catalog, proof)?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)
        .map_err(|error| format!("global constraint proof values invalid: {error}"))?;
    let challenges = derive_global_constraint_challenges(schedule, proof, public_values)?;
    let group_values = load_group_values(catalog, proof)?;
    validate_global_constraint_program(
        &catalog.global_constraints,
        GlobalConstraintInputs {
            publics: &publics,
            proof_values: &packed_proof_values,
            challenges: &challenges,
            group_values: &group_values,
        },
    )
    .map_err(|error| match error {
        GlobalConstraintValidationError::Eval(source) => {
            format!("invalid global constraint program: {source}")
        }
        source => source.to_string(),
    })
}

fn validate_global_hints(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<(), String> {
    if catalog.global_hints.hints.is_empty() {
        return Ok(());
    }

    let requirements = global_hint_input_requirements(&catalog.global_hints);
    let publics = if requirements.publics {
        transcript_public_value_fields(public_values)?
    } else {
        Vec::new()
    };
    let packed_proof_values = if requirements.proof_values {
        let proof_values = load_pcs_proof_values(catalog, proof)?;
        flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)
            .map_err(|error| format!("global hint proof values invalid: {error}"))?
    } else {
        Vec::new()
    };
    let challenges = if requirements.challenges {
        derive_global_constraint_challenges(schedule, proof, public_values)?
    } else {
        Vec::new()
    };
    let group_values = if requirements.group_values {
        load_group_values(catalog, proof)?
    } else {
        Vec::new()
    };

    resolve_global_hint_program(
        &catalog.layout.global_info,
        &catalog.global_hints,
        GlobalConstraintInputs {
            publics: &publics,
            proof_values: &packed_proof_values,
            challenges: &challenges,
            group_values: &group_values,
        },
    )
    .map(|_| ())
    .map_err(|error| format!("invalid global hint program: {error}"))
}

fn derive_global_constraint_challenges(
    schedule: &ProveSchedule,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<Vec<Ext3>, String> {
    if !uses_transcript_query_plan_inputs(proof) {
        return Ok(Vec::new());
    }

    let public_value_fields = transcript_public_value_fields(public_values)?;
    derive_pcs_transcript_challenges_from_proof_segments(
        schedule,
        &public_value_fields,
        &proof.segments,
    )
    .map_err(|error| error.to_string())
}

fn load_pcs_proof_values(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
) -> Result<Vec<Ext3>, String> {
    load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
        .map_err(|error| error.to_string())
}

fn load_group_values(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
) -> Result<Vec<Ext3>, String> {
    load_group_values_from_segments(&catalog.layout.global_info, &proof.segments)
        .map_err(|error| error.to_string())
}

fn validate_setup_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => {
            let fixed_bytes = catalog
                .units
                .iter()
                .map(|unit| unit.actual_fixed_bytes)
                .sum::<u64>();
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", catalog.units.len());
            let _ = writeln!(
                stdout,
                "global_constraints={}",
                catalog.global_constraints.entries.len()
            );
            let _ = writeln!(stdout, "fixed_bytes={fixed_bytes}");
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup validation failed: {error}");
            1
        }
    }
}

fn write_pcs_setup_plan(
    setup_info_bin: &str,
    out_pcs_plan: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let plan = match derive_pcs_setup_plan(&setup) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_pcs_setup_plan(&plan) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    };
    let output = Path::new(out_pcs_plan);
    if let Some(parent) = output.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
            return 1;
        }
    }
    if let Err(error) = std::fs::write(output, &bytes) {
        let _ = writeln!(stderr, "setup PCS plan write failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn write_pcs_setup_material(
    setup_info_bin: &str,
    pcs_plan: &str,
    fixed_const: &str,
    consttree: &str,
    out_pcs_material: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let plan = match read_pcs_setup_plan_file(pcs_plan) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let expected_plan = match derive_pcs_setup_plan(&setup) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    if plan != expected_plan {
        let _ = writeln!(
            stderr,
            "setup PCS material write failed: PCS setup plan does not match setup metadata"
        );
        return 1;
    }
    let fixed_bytes = match std::fs::read(fixed_const) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let tree = match read_constant_tree_file(consttree, &setup) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let material = match build_pcs_setup_material(&plan, &fixed_bytes, &tree) {
        Ok(material) => material,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_pcs_setup_material(&material) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    };
    let output = Path::new(out_pcs_material);
    if let Some(parent) = output.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = writeln!(stderr, "setup PCS material write failed: {error}");
            return 1;
        }
    }
    if let Err(error) = std::fs::write(output, &bytes) {
        let _ = writeln!(stderr, "setup PCS material write failed: {error}");
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn write_fixed_columns_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn write_base_native(
    setup_info_bin: &str,
    columns_bin: &str,
    out_const: &str,
    out_consttree: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file_for_setup(columns_bin, &setup, "raw", "unit") {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let tree = match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)
    {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let fixed_report = match write_base_fixed_columns(out_const, &columns, &setup) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };
    let tree_report = match write_base_constant_tree(out_consttree, &tree, &setup, None) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base write failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "fixed_bytes={}", fixed_report.bytes_written);
    let _ = writeln!(stdout, "tree_bytes={}", tree_report.bytes_written);
    let _ = writeln!(stdout, "root={}", format_root(&tree_report.root));
    let _ = writeln!(stdout, "fixed_output={}", fixed_report.path.display());
    let _ = writeln!(stdout, "tree_output={}", tree_report.path.display());
    0
}

fn write_base_directory(
    setup_dir: &str,
    backend: FixedExtensionBackend,
    derive_verkey: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let layout = match read_key_directory_layout(setup_dir) {
        Ok(layout) => layout,
        Err(error) => {
            let _ = writeln!(stderr, "setup native base directory write failed: {error}");
            return 1;
        }
    };
    if let Err(error) = validate_base_directory_inputs(&layout, derive_verkey) {
        let _ = writeln!(stderr, "setup native base directory write failed: {error}");
        return 1;
    }
    if let Err(error) =
        write_global_info_binary_for_directory(Path::new(setup_dir), &layout.global_info)
    {
        let _ = writeln!(stderr, "setup native base directory write failed: {error}");
        return 1;
    }

    let mut fixed_bytes = 0_u64;
    let mut tree_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    for unit in &layout.units {
        let setup_path = match unit.setup_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup native base directory write failed: missing unit setup metadata path"
                );
                return 1;
            }
        };
        let setup = match read_unit_setup_info_binary_file(&setup_path) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        if let Some(path) = unit.setup_info_binary() {
            if let Err(error) = write_unit_setup_info_binary_for_directory(&path, &setup) {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        }
        let expression_path = match unit.expression_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup native base directory write failed: missing unit expression metadata path"
                );
                return 1;
            }
        };
        let expressions = match read_expression_info_binary_file(&expression_path) {
            Ok(expressions) => expressions,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        if let Some(path) = unit.expression_info_binary() {
            if let Err(error) = write_expression_info_binary_for_directory(&path, &expressions) {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        }
        if let Some(path) = unit.expression_program() {
            if let Err(error) = write_regular_hint_program_for_directory(&path, &expressions) {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        }
        let verifier_path = match unit.verifier_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup native base directory write failed: missing unit verifier metadata path"
                );
                return 1;
            }
        };
        let verifier = match read_verifier_info_binary_file(&verifier_path) {
            Ok(verifier) => verifier,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        if let Some(path) = unit.verifier_info_binary() {
            if let Err(error) = write_verifier_info_binary_for_directory(&path, &verifier) {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        }
        let group_name = unit.group_name.as_deref().unwrap_or("raw");
        let unit_name = unit.unit_name.as_deref().unwrap_or("unit");
        let columns = match read_fixed_columns_file_for_setup(
            &unit.fixed_columns,
            &setup,
            group_name,
            unit_name,
        ) {
            Ok(columns) => columns,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        let expected_root = if derive_verkey {
            None
        } else {
            match read_verification_key_binary_file(unit.verification_key_binary()) {
                Ok(root) => Some(root),
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            }
        };
        let tree =
            match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend) {
                Ok(tree) => tree,
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            };
        let fixed_report = match write_base_fixed_columns(&unit.fixed_columns, &columns, &setup) {
            Ok(report) => report,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        let tree_report = match write_base_constant_tree(
            &unit.constant_tree,
            &tree,
            &setup,
            expected_root.as_ref(),
        ) {
            Ok(report) => report,
            Err(error) => {
                let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                return 1;
            }
        };
        if derive_verkey {
            let key_report = match write_verification_key_from_constant_tree(
                unit.verification_key_binary(),
                &tree,
                &setup,
            ) {
                Ok(report) => report,
                Err(error) => {
                    let _ = writeln!(stderr, "setup native base directory write failed: {error}");
                    return 1;
                }
            };
            verkey_bytes = verkey_bytes.saturating_add(key_report.binary_bytes);
        }

        fixed_bytes = fixed_bytes.saturating_add(fixed_report.bytes_written);
        tree_bytes = tree_bytes.saturating_add(tree_report.bytes_written);
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", layout.units.len());
    let _ = writeln!(stdout, "fixed_bytes={fixed_bytes}");
    let _ = writeln!(stdout, "tree_bytes={tree_bytes}");
    if derive_verkey {
        let _ = writeln!(stdout, "verkey_bytes={verkey_bytes}");
    }
    0
}

fn write_global_info_binary_for_directory(
    root: &Path,
    global_info: &GlobalInfo,
) -> Result<u64, String> {
    let bytes = encode_global_info(global_info).map_err(|error| error.to_string())?;
    let output = root.join(GLOBAL_INFO_BINARY_FILE_NAME);
    std::fs::write(&output, &bytes).map_err(|error| {
        format!(
            "write global-info binary failed: {}: {error}",
            output.display()
        )
    })?;
    Ok(bytes.len() as u64)
}

fn write_unit_setup_info_binary_for_directory(
    path: &Path,
    setup: &UnitSetupInfo,
) -> Result<u64, String> {
    let bytes = encode_unit_setup_info(setup).map_err(|error| error.to_string())?;
    std::fs::write(path, &bytes).map_err(|error| {
        format!(
            "write setup metadata binary failed: {}: {error}",
            path.display()
        )
    })?;
    Ok(bytes.len() as u64)
}

fn write_expression_info_binary_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
) -> Result<u64, String> {
    let bytes = encode_expression_info(expressions).map_err(|error| error.to_string())?;
    std::fs::write(path, &bytes).map_err(|error| {
        format!(
            "write expression metadata binary failed: {}: {error}",
            path.display()
        )
    })?;
    Ok(bytes.len() as u64)
}

fn write_regular_hint_program_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
) -> Result<u64, String> {
    let program = regular_hint_program_from_expression_info(expressions)
        .map_err(|error| error.to_string())?;
    let hint_file = encode_regular_hint_program(&program).map_err(|error| error.to_string())?;
    let hint_section = parse_sectioned_file(&hint_file, *b"chps", 1)
        .map_err(|error| error.to_string())?
        .sections
        .into_iter()
        .find(|section| section.id == 3)
        .ok_or_else(|| "encoded hint program is missing hint section".to_owned())?;

    let existing = std::fs::read(path).map_err(|error| {
        format!(
            "read expression program for hint merge failed: {}: {error}",
            path.display()
        )
    })?;
    let mut file = parse_sectioned_file(&existing, *b"chps", 1).map_err(|error| {
        format!(
            "parse expression program for hint merge failed: {}: {error}",
            path.display()
        )
    })?;
    file.sections.retain(|section| section.id != 3);
    file.sections.push(hint_section);
    file.sections.sort_by_key(|section| section.id);
    let bytes = encode_sectioned_file(&file).map_err(|error| error.to_string())?;
    std::fs::write(path, &bytes).map_err(|error| {
        format!(
            "write expression program hint section failed: {}: {error}",
            path.display()
        )
    })?;
    Ok(bytes.len() as u64)
}

fn write_verifier_info_binary_for_directory(
    path: &Path,
    verifier: &VerifierInfo,
) -> Result<u64, String> {
    let bytes = encode_verifier_info(verifier).map_err(|error| error.to_string())?;
    std::fs::write(path, &bytes).map_err(|error| {
        format!(
            "write verifier metadata binary failed: {}: {error}",
            path.display()
        )
    })?;
    Ok(bytes.len() as u64)
}

fn write_pcs_directory(setup_dir: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let layout = match read_key_directory_layout(setup_dir) {
        Ok(layout) => layout,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
            return 1;
        }
    };

    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let setup_path = match unit.setup_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS directory write failed: missing unit setup metadata path"
                );
                return 1;
            }
        };
        let output = match unit.pcs_setup_plan() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS directory write failed: missing unit PCS plan output path"
                );
                return 1;
            }
        };
        let setup = match read_unit_setup_info_binary_file(&setup_path) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        let plan = match derive_pcs_setup_plan(&setup) {
            Ok(plan) => plan,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        let bytes = match encode_pcs_setup_plan(&plan) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        };
        if let Some(parent) = output.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
                return 1;
            }
        }
        if let Err(error) = std::fs::write(&output, &bytes) {
            let _ = writeln!(stderr, "setup PCS directory write failed: {error}");
            return 1;
        }
        bytes_written = bytes_written.saturating_add(bytes.len() as u64);
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", layout.units.len());
    let _ = writeln!(stdout, "bytes_written={bytes_written}");
    0
}

fn write_pcs_material_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let layout = match read_key_directory_layout(setup_dir) {
        Ok(layout) => layout,
        Err(error) => {
            let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
            return 1;
        }
    };

    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let setup_path = match unit.setup_info() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS material directory write failed: missing unit setup metadata path"
                );
                return 1;
            }
        };
        let plan_path = match unit.pcs_setup_plan() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS material directory write failed: missing unit PCS plan path"
                );
                return 1;
            }
        };
        let output = match unit.pcs_setup_material() {
            Some(path) => path,
            None => {
                let _ = writeln!(
                    stderr,
                    "setup PCS material directory write failed: missing unit PCS material output path"
                );
                return 1;
            }
        };
        let setup = match read_unit_setup_info_binary_file(&setup_path) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        let plan = match read_pcs_setup_plan_file(&plan_path) {
            Ok(plan) => plan,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        let expected_plan = match derive_pcs_setup_plan(&setup) {
            Ok(plan) => plan,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        if plan != expected_plan {
            let _ = writeln!(
                stderr,
                "setup PCS material directory write failed: PCS setup plan does not match setup metadata"
            );
            return 1;
        }
        let fixed_bytes = match std::fs::read(&unit.fixed_columns) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        let tree = match read_constant_tree_file(&unit.constant_tree, &setup) {
            Ok(tree) => tree,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        let material = match build_pcs_setup_material(&plan, &fixed_bytes, &tree) {
            Ok(material) => material,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        let bytes = match encode_pcs_setup_material(&material) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        };
        if let Some(parent) = output.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
                return 1;
            }
        }
        if let Err(error) = std::fs::write(&output, &bytes) {
            let _ = writeln!(stderr, "setup PCS material directory write failed: {error}");
            return 1;
        }
        bytes_written = bytes_written.saturating_add(bytes.len() as u64);
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", layout.units.len());
    let _ = writeln!(stdout, "bytes_written={bytes_written}");
    0
}

fn validate_base_directory_inputs(
    layout: &lzvm_artifacts::key_directory::KeyDirectoryLayout,
    derive_verkey: bool,
) -> Result<(), String> {
    if !derive_verkey {
        return validate_key_directory_layout(layout).map_err(|error| error.to_string());
    }

    let mut seen = BTreeSet::new();
    for required in layout.required_paths() {
        if matches!(
            required.role,
            "unit verification-key metadata" | "unit verification-key binary"
        ) {
            continue;
        }
        if !seen.insert(required.path.clone()) {
            continue;
        }
        if !required.path.is_file() {
            return Err(format!(
                "missing key-directory {}: {}",
                required.role,
                required.path.display()
            ));
        }
    }
    Ok(())
}

fn write_verification_key_native(
    setup_info_bin: &str,
    consttree: &str,
    out_verkey_bin: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            return 1;
        }
    };
    let tree = match std::fs::read(consttree) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            return 1;
        }
    };

    match write_verification_key_from_constant_tree(out_verkey_bin, &tree, &setup) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "binary_bytes={}", report.binary_bytes);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "binary_output={}", report.binary_path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup verification-key write failed: {error}");
            1
        }
    }
}

fn write_constant_tree(
    setup_info_bin: &str,
    tree_bin: &str,
    root_bin: &str,
    out_consttree: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };
    let tree = match std::fs::read(tree_bin) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };
    let root = match read_verification_key_binary_file(root_bin) {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            return 1;
        }
    };

    match write_base_constant_tree(out_consttree, &tree, &setup, Some(&root)) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree write failed: {error}");
            1
        }
    }
}

fn write_constant_tree_leaves_command(
    setup_info_bin: &str,
    columns_bin: &str,
    out_leaves: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            return 1;
        }
    };

    match write_constant_tree_leaves_with_backend(out_leaves, &columns, &setup, backend) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "rows={}", report.row_count);
            let _ = writeln!(stdout, "columns={}", report.column_count);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup constant-tree leaf write failed: {error}");
            1
        }
    }
}

fn write_constant_tree_native(
    setup_info_bin: &str,
    columns_bin: &str,
    root_bin: Option<&str>,
    out_consttree: &str,
    backend: FixedExtensionBackend,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_binary_file(setup_info_bin) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file_for_setup(columns_bin, &setup, "raw", "unit") {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };
    let expected_root = match root_bin {
        Some(path) => match read_verification_key_binary_file(path) {
            Ok(root) => Some(root),
            Err(error) => {
                let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
                return 1;
            }
        },
        None => None,
    };

    let tree = match build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)
    {
        Ok(tree) => tree,
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            return 1;
        }
    };

    match write_base_constant_tree(out_consttree, &tree, &setup, expected_root.as_ref()) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "root={}", format_root(&report.root));
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup native constant-tree write failed: {error}");
            1
        }
    }
}

fn publish_fixed_columns(
    out_const: &str,
    columns: &FixedColumns,
    setup: &lzvm_artifacts::setup_info::UnitSetupInfo,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match write_base_fixed_columns(out_const, columns, setup) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            1
        }
    }
}

fn parse_fixed_extension_backend(
    value: &str,
    role: &str,
    stderr: &mut dyn Write,
) -> Option<FixedExtensionBackend> {
    match value {
        "cpu" => Some(FixedExtensionBackend::Cpu),
        "cuda" => Some(FixedExtensionBackend::Cuda),
        _ => {
            let _ = writeln!(stderr, "{role} failed: unsupported backend {value}");
            None
        }
    }
}

fn write_validate_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup validate <setup-dir>");
    2
}

fn write_verify_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify preflight <proof-bin> <public-values>"
    );
    2
}

fn write_verify_setup_preflight_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm verify setup-preflight <setup-dir> <proof-bin> <public-values>"
    );
    2
}

fn write_prove_schedule_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm prove schedule <setup-dir>");
    2
}

fn write_fingerprint_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup fingerprint <setup-dir>");
    2
}

fn write_pcs_plan_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-plan <setup-info-bin> <out-pcs-plan>"
    );
    2
}

fn write_pcs_material_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-material <setup-info-bin> <pcs-plan> <fixed-const> <consttree> <out-pcs-material>"
    );
    2
}

fn write_fixed_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed-native <setup-info-bin> <columns-bin> <out-const>"
    );
    2
}

fn write_base_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-base-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-const> <out-consttree>"
    );
    2
}

fn write_base_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-base-directory [--derive-verkey] [--backend cpu|cuda] <setup-dir>"
    );
    2
}

fn write_pcs_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup write-pcs-directory <setup-dir>");
    2
}

fn write_pcs_material_directory_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-pcs-material-directory <setup-dir>"
    );
    2
}

fn write_verkey_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-verkey-native <setup-info-bin> <consttree> <out-verkey-bin>"
    );
    2
}

fn write_const_tree_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-tree <setup-info-bin> <tree-bin> <root-bin> <out-consttree>"
    );
    2
}

fn write_const_leaves_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-leaves [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-leaves>"
    );
    2
}

fn write_const_native_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-const-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> [root-bin] <out-consttree>"
    );
    2
}

fn format_hash(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn format_root(root: &VerificationKeyRoot) -> String {
    match root {
        VerificationKeyRoot::FieldElements(values) => values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(","),
    }
}
