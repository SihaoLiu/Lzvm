use std::fs;
use std::path::Path;

use lzvm_artifacts::eth_block_public_values::{
    public_values_from_eth_block_input_for_metadata,
    validate_eth_block_public_values_with_program_image_cache,
};
use lzvm_artifacts::key_directory::{key_directory_catalog_digest, KeyDirectoryCatalog};
use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache,
};
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, read_public_values_file, PublicValues,
};
use lzvm_prover::ProveExecutionInputArtifacts;

use crate::eth_block_prove_input::{
    validate_eth_block_input, write_eth_block_input_from_public_input, EthBlockInputSummary,
    EthPublicInputMode,
};

use super::args::{parsed_inputs, ParsedWitnessArgs};

pub(super) struct PreparedPublicInputs {
    pub(super) inputs: ProveExecutionInputArtifacts,
    pub(super) generated: bool,
}

pub(super) struct PreparedEthBlockInput {
    pub(super) summary: Option<EthBlockInputSummary>,
    pub(super) generated_from_public_input: bool,
}

pub(super) struct PublicInputSummary {
    pub(super) digest: [u8; 32],
    pub(super) value_count: usize,
    pub(super) field_count: usize,
}

pub(super) fn summarize_public_inputs(
    path: Option<&Path>,
) -> Result<Option<PublicInputSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let public_values = read_public_values_file(path)
        .map_err(|error| format!("read public inputs failed: {}: {error}", path.display()))?;
    let digest = public_values_digest(&public_values)
        .map_err(|error| format!("digest public inputs failed: {}: {error}", path.display()))?;
    Ok(Some(PublicInputSummary {
        digest,
        value_count: public_values.values.len(),
        field_count: public_values_field_count(&public_values),
    }))
}

pub(super) fn public_values_field_count(public_values: &PublicValues) -> usize {
    public_values
        .values
        .iter()
        .map(|entry| entry.elements.len())
        .sum()
}

pub(super) fn prepare_eth_block_input(
    parsed: &ParsedWitnessArgs,
) -> Result<PreparedEthBlockInput, String> {
    if let Some(path) = &parsed.eth_block_input {
        return Ok(PreparedEthBlockInput {
            summary: validate_eth_block_input(&Some(path.clone()))?,
            generated_from_public_input: false,
        });
    }
    let Some(path) = &parsed.eth_public_input else {
        return Ok(PreparedEthBlockInput {
            summary: None,
            generated_from_public_input: false,
        });
    };

    let output_path = parsed.run_args.positionals[1].join("eth-block.input");
    let mode = if parsed.eth_public_input_allow_trailing {
        EthPublicInputMode::AllowTrailing
    } else {
        EthPublicInputMode::Strict
    };
    Ok(PreparedEthBlockInput {
        summary: Some(write_eth_block_input_from_public_input(
            path,
            &output_path,
            mode,
        )?),
        generated_from_public_input: true,
    })
}

pub(super) fn prepare_eth_block_public_inputs(
    parsed: &ParsedWitnessArgs,
    catalog: &KeyDirectoryCatalog,
    eth_block_input: Option<&EthBlockInputSummary>,
) -> Result<PreparedPublicInputs, String> {
    let mut inputs = parsed_inputs(parsed);
    let Some(summary) = eth_block_input else {
        return Ok(PreparedPublicInputs {
            inputs,
            generated: false,
        });
    };
    let setup_hash = key_directory_catalog_digest(catalog)
        .map_err(|error| format!("derive setup hash failed: {error}"))?;
    if let Some(public_inputs) = &inputs.public_inputs {
        let public_values = read_public_values_file(public_inputs).map_err(|error| {
            format!(
                "read public inputs failed: {}: {error}",
                public_inputs.display()
            )
        })?;
        if public_values.setup_hash != setup_hash {
            return Err("public inputs setup hash mismatch".to_owned());
        }
        let program_image_cache =
            read_optional_program_image_cache(parsed.run_args.program_image_cache.as_deref())?;
        validate_eth_block_public_values_with_program_image_cache(
            &summary.input,
            &public_values,
            program_image_cache.as_ref(),
        )
        .map_err(|error| error.to_string())?;
        return Ok(PreparedPublicInputs {
            inputs,
            generated: false,
        });
    }

    let output_dir = &parsed.run_args.positionals[1];
    let public_inputs = output_dir.join("eth-block-public-values.bin");
    let program_image_cache =
        read_optional_program_image_cache(parsed.run_args.program_image_cache.as_deref())?;
    let public_values = public_values_from_eth_block_input_for_metadata(
        setup_hash,
        &summary.input,
        &catalog.layout.global_info,
        program_image_cache.as_ref(),
    )
    .map_err(|error| format!("encode ETH block public values failed: {error}"))?;
    let encoded = encode_public_values(&public_values)
        .map_err(|error| format!("encode ETH block public values failed: {error}"))?;
    if let Some(parent) = public_inputs.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "create public inputs directory failed: {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    fs::write(&public_inputs, encoded).map_err(|error| {
        format!(
            "write ETH block public values failed: {}: {error}",
            public_inputs.display()
        )
    })?;
    inputs.public_inputs = Some(public_inputs);
    Ok(PreparedPublicInputs {
        inputs,
        generated: true,
    })
}

fn read_optional_program_image_cache(
    path: Option<&Path>,
) -> Result<Option<ProgramImageCommitmentCache>, String> {
    path.map(|path| {
        read_program_image_commitment_cache_file(path).map_err(|error| {
            format!(
                "read program-image cache failed: {}: {error}",
                path.display()
            )
        })
    })
    .transpose()
}
