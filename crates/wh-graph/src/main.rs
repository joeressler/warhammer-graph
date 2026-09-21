use std::path::PathBuf;
use std::process::ExitCode;

use clap::{error::ErrorKind, Parser, Subcommand};
use wh_graph::{build_bundle, build_example, render_summary, validate_bundle, validate_example};

const EXAMPLES: &str = "\
Examples:
  wh-graph build --corpus ./corpus --out ./bundle
  wh-graph build --corpus ./corpus --out ./bundle --dry-run --output json
  wh-graph validate --bundle ./bundle";

#[derive(Parser)]
#[command(
    name = "wh-graph",
    about = "Build a petgraph bundle from a Wahapedia corpus v1 directory.",
    after_help = EXAMPLES
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read corpus v1 and write a petgraph bundle.
    #[command(after_help = "\
Examples:
  wh-graph build --corpus ./corpus --out ./bundle
  wh-graph build --corpus ./corpus --out ./bundle --dry-run --output json")]
    Build {
        /// Corpus v1 directory (`manifest.json` and `entities.jsonl`).
        #[arg(long, value_name = "PATH")]
        corpus: PathBuf,
        /// Bundle directory to write.
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
        /// Parse and build, then print counts without writing `--out`.
        #[arg(long)]
        dry_run: bool,
        /// `text` or `json`.
        #[arg(long, default_value = "text", value_name = "text|json")]
        output: String,
    },
    /// Check a petgraph bundle against the graph rules.
    #[command(after_help = "\
Examples:
  wh-graph validate --bundle ./bundle")]
    Validate {
        /// Bundle directory to read.
        #[arg(long, value_name = "PATH")]
        bundle: PathBuf,
        /// `text` or `json`.
        #[arg(long, default_value = "text", value_name = "text|json")]
        output: String,
    },
}

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => match dispatch(cli) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(err.exit_code() as u8)
            }
        },
        Err(err) => {
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                err.print().ok();
                ExitCode::SUCCESS
            } else {
                eprintln!("{err}");
                let example = if std::env::args().any(|arg| arg == "validate") {
                    validate_example()
                } else {
                    build_example()
                };
                eprintln!("{example}");
                ExitCode::from(2)
            }
        }
    }
}

fn dispatch(cli: Cli) -> Result<String, wh_graph::GraphError> {
    match cli.command {
        Command::Build {
            corpus,
            out,
            dry_run,
            output,
        } => {
            let example = build_example();
            require_output(&output, example)?;
            let summary = build_bundle(&corpus, &out, &out.display().to_string(), dry_run)?;
            render_summary(&summary, &output, example)
        }
        Command::Validate { bundle, output } => {
            let example = validate_example();
            require_output(&output, example)?;
            let summary = validate_bundle(&bundle, &bundle.display().to_string())?;
            render_summary(&summary, &output, example)
        }
    }
}

fn require_output(output: &str, example: &str) -> Result<(), wh_graph::GraphError> {
    if output == "text" || output == "json" {
        Ok(())
    } else {
        Err(wh_graph::GraphError::Usage(format!(
            "bad flag: --output\n{example}"
        )))
    }
}
