use lzvm_cli::run_cli;

#[test]
fn top_level_help_prints_command_groups_to_stdout() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = run_cli(&["--help"], &mut stdout, &mut stderr);

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("usage: lzvm <group> <command> [args]\n"));
    assert!(stdout.contains("  eth     block input and public input helpers\n"));
    assert!(stdout.contains("  prove   plan, inputs, witness, and schedule commands\n"));
    assert!(stdout.contains("  verify  setup preflight, proof, and contribution checks\n"));
}

#[test]
fn empty_cli_prints_top_level_usage_to_stderr() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = run_cli(&[], &mut stdout, &mut stderr);

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("usage: lzvm <group> <command> [args]\n"));
    assert!(!stderr.contains("usage: lzvm setup validate <setup-dir>\n"));
}

#[test]
fn unknown_cli_command_prints_top_level_usage_to_stderr() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = run_cli(&["unknown"], &mut stdout, &mut stderr);

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("usage: lzvm <group> <command> [args]\n"));
    assert!(!stderr.contains("usage: lzvm setup validate <setup-dir>\n"));
}
