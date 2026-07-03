use lzvm_cli::run_cli;

fn expected_top_level_usage() -> &'static str {
    concat!(
        "usage: lzvm <group> <command> [args]\n",
        "\n",
        "groups:\n",
        "  eth     block input and public input helpers\n",
        "  pil     PIL archive, summary, and graph helpers\n",
        "  prove   plan, inputs, witness, guest-run, and schedule commands\n",
        "  setup   validate, fingerprint, and setup artifact writers\n",
        "  verify  setup preflight, proof, and contribution checks\n",
    )
}

#[test]
fn top_level_help_prints_command_groups_to_stdout() {
    for args in [["--help"], ["-h"], ["help"]] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli(&args, &mut stdout, &mut stderr);

        assert_eq!(code, 0, "help args should succeed: {args:?}");
        assert!(
            stderr.is_empty(),
            "help args should not write stderr: {args:?}"
        );
        let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
        assert_eq!(stdout, expected_top_level_usage(), "help args: {args:?}");
    }
}

#[test]
fn empty_cli_prints_top_level_usage_to_stderr() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = run_cli(&[], &mut stdout, &mut stderr);

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert_eq!(stderr, expected_top_level_usage());
}

#[test]
fn unknown_cli_command_prints_top_level_usage_to_stderr() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = run_cli(&["unknown"], &mut stdout, &mut stderr);

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert_eq!(stderr, expected_top_level_usage());
}
