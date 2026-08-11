//! Thin entry point: run the CLI, map an error onto an exit code, print nothing else.

use notiflow::cli;
use notiflow::error::Rendered;

fn main() {
    match cli::dispatch() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            notiflow::safe_eprintln!("{}", Rendered(&err));
            std::process::exit(err.exit_code());
        }
    }
}
