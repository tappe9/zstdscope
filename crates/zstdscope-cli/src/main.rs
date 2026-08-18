#![forbid(unsafe_code)]

mod render;

use std::{
    fmt, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use zstdscope::{ZstdError, ZstdFile};

#[derive(Debug, Parser)]
#[command(
    name = "zstdscope",
    version,
    about = "Inspect Zstandard compressed data"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect the structural metadata of a Zstandard file.
    Inspect {
        file: PathBuf,
        /// Emit machine-readable JSON instead of the human-readable summary.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    match run(Cli::parse(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is_broken_pipe() => ExitCode::SUCCESS,
        Err(error) => {
            let stderr = io::stderr();
            let mut stderr = stderr.lock();
            let _ = writeln!(stderr, "{error}");
            ExitCode::FAILURE
        }
    }
}

fn run<W: Write>(cli: Cli, writer: &mut W) -> Result<(), CliError> {
    match cli.command {
        Command::Inspect { file, json } => inspect_file(file, json, writer),
    }
}

fn inspect_file<W: Write>(path: PathBuf, json: bool, writer: &mut W) -> Result<(), CliError> {
    let input = fs::read(&path).map_err(|source| CliError::Io {
        path: path.clone(),
        source,
    })?;
    let file = zstdscope::inspect(&input).map_err(CliError::Parse)?;
    write_inspection(writer, &file, json)
}

fn write_inspection<W: Write>(
    writer: &mut W,
    file: &ZstdFile,
    json: bool,
) -> Result<(), CliError> {
    if json {
        serde_json::to_writer_pretty(&mut *writer, file).map_err(CliError::Json)?;
        writer.write_all(b"\n").map_err(CliError::Output)?;
    } else {
        render::render(writer, file).map_err(CliError::Output)?;
    }

    Ok(())
}

#[derive(Debug)]
enum CliError {
    Io { path: PathBuf, source: io::Error },
    Parse(ZstdError),
    Json(serde_json::Error),
    Output(io::Error),
}

impl CliError {
    fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Json(source) => source.io_error_kind() == Some(io::ErrorKind::BrokenPipe),
            Self::Output(source) => source.kind() == io::ErrorKind::BrokenPipe,
            Self::Io { .. } | Self::Parse(_) => false,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "I/O error: failed to read {}: {source}",
                    path.display()
                )
            }
            Self::Parse(source) => write!(formatter, "parse error: {source}"),
            Self::Json(source) => write!(formatter, "JSON serialization error: {source}"),
            Self::Output(source) => write!(formatter, "output error: {source}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse(source) => Some(source),
            Self::Json(source) => Some(source),
            Self::Output(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingWriter {
        kind: io::ErrorKind,
    }

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.kind))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn minimal_file() -> ZstdFile {
        zstdscope::inspect(&[0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x01, 0x00, 0x00]).unwrap()
    }

    #[test]
    fn text_output_write_failure_is_typed() {
        let mut writer = FailingWriter {
            kind: io::ErrorKind::Other,
        };
        let error = write_inspection(&mut writer, &minimal_file(), false).unwrap_err();

        assert!(matches!(error, CliError::Output(_)));
    }

    #[test]
    fn json_broken_pipe_is_classified_as_normal_pipe_closure() {
        let mut writer = FailingWriter {
            kind: io::ErrorKind::BrokenPipe,
        };
        let error = write_inspection(&mut writer, &minimal_file(), true).unwrap_err();

        assert!(error.is_broken_pipe());
    }
}
