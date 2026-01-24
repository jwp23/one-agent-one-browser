mod platform;

use std::path::PathBuf;

fn main() {
    let _html_path = std::env::args_os().nth(1).map(PathBuf::from);
    if let Err(err) = platform::run_hello_world_window() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

