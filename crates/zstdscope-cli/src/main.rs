#![forbid(unsafe_code)]

mod render;

use std::{
    fmt,
    fs,
    io,
    path::PathBuf,
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use zstdscope::ZstdError;

#[derive(Debug, Parser)]
#[command(name = "zstdscope", version, about = "Inspect Zstandard compressed data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect the structural metadata of a Zstandard file.
    Inspect { file: PathBuf },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Inspect { file } => inspect_file(file),
    }
}

fn inspect_file(path: PathBuf) -> Result<(), CliError> {
    let input = fs::read(&path).map_err(|source| CliError::Io {
        path: path.clone(),
        source,
    })?;
    let file = zstdscope::inspect(&input).map_err(CliError::Parse)?;
    print!("{}", render::render(&file));
    Ok(())
}

#[derive(Debug)]
enum CliError {
    Io { path: PathBuf, source: io::Error },
    Parse(ZstdError),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "I/O error: failed to read {}: {source}", path.display())
            }
            Self::Parse(source) => write!(formatter, "parse error: {source}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse(source) => Some(source),
        }
    }
}
