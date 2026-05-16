use std::io::Write;
use std::path::PathBuf;

use lzvm_pil::SourceProgramArchiveLoader;

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let archive_path = match args {
        [archive_path] => PathBuf::from(archive_path),
        _ => return write_usage(stderr),
    };

    let program = match SourceProgramArchiveLoader::load(&archive_path) {
        Ok(program) => program,
        Err(error) => {
            let _ = writeln!(stderr, "pil archive-summary failed: {error}");
            return 1;
        }
    };

    crate::pil_summary::write_summary(stdout, &program);
    0
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm pil archive-summary <archive-file>");
    2
}
