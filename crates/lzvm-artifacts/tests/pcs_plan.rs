use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, parse_pcs_setup_plan, read_pcs_setup_plan_file,
    PcsPlanError,
};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 5,
        "nPublics": 3,
        "nConstraints": 8,
        "qDeg": 7,
        "openingPoints": [0, 1, -1],
        "mapSectionsN": {
            "const": 5,
            "cm1": 2,
            "cm2": 3,
            "cm3": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.a", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.b", "dim": 1, "polsMapId": 1, "stageId": 1},
            {"stage": 0, "name": "main.c", "dim": 1, "polsMapId": 2, "stageId": 2},
            {"stage": 0, "name": "main.d", "dim": 1, "polsMapId": 3, "stageId": 3},
            {"stage": 0, "name": "main.e", "dim": 1, "polsMapId": 4, "stageId": 4, "lengths": [5]}
        ],
        "challengesMap": [{}, {}],
        "evMap": [{}, {}, {}],
        "boundaries": [
            {"name": "first", "offsetMin": 0, "offsetMax": 3},
            {"offsetMin": -1}
        ],
        "starkStruct": {
            "nBits": 10,
            "nBitsExt": 13,
            "nQueries": 4,
            "steps": [
                {"nBits": 13},
                {"nBits": 9},
                {"nBits": 5}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 20,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-pcs-plan-{}-{name}", std::process::id()))
}

#[test]
fn derives_pcs_setup_plan_from_unit_setup_metadata() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");

    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");

    assert_eq!(plan.base_domain_bits, 10);
    assert_eq!(plan.extended_domain_bits, 13);
    assert_eq!(plan.base_domain_size, 1024);
    assert_eq!(plan.extended_domain_size, 8192);
    assert_eq!(plan.blowup_factor, 8);
    assert_eq!(plan.query_count, 4);
    assert_eq!(plan.proof_of_work_bits, 20);
    assert_eq!(plan.merkle_tree_arity, 4);
    assert_eq!(plan.transcript_arity, Some(4));
    assert_eq!(plan.constant_width, 5);
    assert_eq!(plan.stage_commit_widths, vec![2, 3, 1]);
    assert_eq!(plan.opening_points, vec![0, 1, -1]);
    assert_eq!(plan.final_layer_bits, 5);
    assert_eq!(plan.fri_layers.len(), 2);
    assert_eq!(plan.fri_layers[0].input_bits, 13);
    assert_eq!(plan.fri_layers[0].output_bits, 9);
    assert_eq!(plan.fri_layers[0].folding_factor, 16);
    assert_eq!(plan.fri_layers[1].input_bits, 9);
    assert_eq!(plan.fri_layers[1].output_bits, 5);
    assert_eq!(plan.fri_layers[1].folding_factor, 16);
}

#[test]
fn encodes_and_parses_pcs_setup_plans() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");

    let encoded = encode_pcs_setup_plan(&plan).expect("plan should encode");
    let parsed = parse_pcs_setup_plan(&encoded).expect("plan should parse");

    assert_eq!(&encoded[0..4], b"pcsp");
    assert_eq!(parsed, plan);
}

#[test]
fn reads_pcs_setup_plans_from_a_file_path() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let path = temp_file_path("plan.bin");
    fs::write(
        &path,
        encode_pcs_setup_plan(&plan).expect("plan should encode"),
    )
    .expect("fixture should be written");

    let parsed = read_pcs_setup_plan_file(&path).expect("plan should read");
    fs::remove_file(&path).expect("fixture should be removed");

    assert_eq!(parsed, plan);
}

#[test]
fn rejects_invalid_pcs_folding_schedule() {
    let mut setup =
        parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    setup.stark.steps[1].n_bits = 13;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::InvalidFriLayer {
            input_bits: 13,
            output_bits: 13
        })
    ));
}

#[test]
fn rejects_pcs_domains_that_do_not_fit_u64() {
    let mut setup =
        parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    setup.stark.n_bits_ext = 64;
    setup.stark.steps[0].n_bits = 64;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::DomainTooLarge { bits: 64 })
    ));
}

#[test]
fn rejects_pcs_domains_that_shrink_before_extension() {
    let mut setup =
        parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    setup.stark.n_bits_ext = 9;
    setup.stark.steps[0].n_bits = 9;

    assert!(matches!(
        derive_pcs_setup_plan(&setup),
        Err(PcsPlanError::InvalidDomainBits {
            base_bits: 10,
            extended_bits: 9
        })
    ));
}
