mod cli;
mod error;
mod shadow {
    #![allow(clippy::needless_raw_string_hashes)]
    use shadow_rs::shadow;
    shadow!(build);

    pub use self::build::*;
}

use std::path::Path;

use clap::Parser;

use self::cli::Cli;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let (path, args) =
        args.split_first().expect("Expected at least one argument (the binary name)");
    let args = args.to_vec();
    let bin_name = Path::new(path).file_name().unwrap_or_default().to_str().unwrap_or_default();

    let cli = match bin_name {
        "pause" => Cli::parse_from([String::new(), "idle".to_string()].into_iter().chain(args)),
        "tini" => Cli::parse_from([String::new(), "entry".to_string()].into_iter().chain(args)),
        _ => Cli::default(),
    };

    match cli.run() {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}
