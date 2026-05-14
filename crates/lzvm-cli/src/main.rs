fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    std::process::exit(lzvm_cli::run_cli(&refs, &mut stdout, &mut stderr));
}
