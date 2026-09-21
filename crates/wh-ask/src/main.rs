use std::path::PathBuf;
use std::process::ExitCode;

use clap::{error::ErrorKind, Parser, Subcommand};
use wh_ask::{render_output, AskError, DEFAULT_TOP_K, QUERY_EXAMPLE};

mod llama;
mod ui;

const EXAMPLES: &str = "\
Examples:
  wh-ask
  wh-ask --bundle ./bundle --chat-model ./models/chat.gguf --embed-model ./models/embed.gguf
  wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --question \"Which example units are in Example Faction?\"
  wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --embed-model ./models/embed.gguf --question \"...\" --top-k 8";

const QUERY_EXAMPLES: &str = "\
Examples:
  wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --question \"Which example units are in Example Faction?\"
  wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --embed-model ./models/embed.gguf --question \"...\" --top-k 8";

#[derive(Parser)]
#[command(
    name = "wh-ask",
    about = "Answer questions from a Wahapedia graph bundle.",
    after_help = EXAMPLES
)]
struct Cli {
    /// Bundle directory to prefill on the settings screen.
    #[arg(long, value_name = "PATH")]
    bundle: Option<PathBuf>,
    /// Chat GGUF to prefill on the settings screen.
    #[arg(long, value_name = "PATH")]
    chat_model: Option<PathBuf>,
    /// Embedding GGUF to prefill on the settings screen.
    #[arg(long, value_name = "PATH")]
    embed_model: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Answer one question and exit.
    #[command(after_help = QUERY_EXAMPLES)]
    Query {
        /// Bundle directory (`manifest.json`, JSONL, optional postcard).
        #[arg(long, value_name = "PATH")]
        bundle: PathBuf,
        /// Chat GGUF.
        #[arg(long, value_name = "PATH")]
        chat_model: PathBuf,
        /// Embedding GGUF. Omit it to use label search.
        #[arg(long, value_name = "PATH")]
        embed_model: Option<PathBuf>,
        /// Question to answer from the bundle.
        #[arg(long)]
        question: String,
        /// Passages to retrieve, from 1 to 32.
        #[arg(long)]
        top_k: Option<i64>,
        /// `text` or `json`.
        #[arg(long, default_value = "text", value_name = "text|json")]
        output: String,
    },
}

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => match cli.command {
            Some(command) => match dispatch(command) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::from(err.exit_code() as u8)
                }
            },
            None => match ui::run(ui::Prefill {
                bundle: cli.bundle,
                chat_model: cli.chat_model,
                embed_model: cli.embed_model,
            }) {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::from(1)
                }
            },
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
                eprintln!("{QUERY_EXAMPLE}");
                ExitCode::from(2)
            }
        }
    }
}

fn dispatch(command: Command) -> Result<String, AskError> {
    match command {
        Command::Query {
            bundle,
            chat_model,
            embed_model,
            question,
            top_k,
            output,
        } => {
            if output != "text" && output != "json" {
                return Err(AskError::usage(format!(
                    "bad flag: --output\n{QUERY_EXAMPLE}"
                )));
            }
            let top_k = match top_k {
                None => DEFAULT_TOP_K as usize,
                Some(value) if (1..=32).contains(&value) => value as usize,
                Some(_) => {
                    return Err(AskError::usage(format!(
                        "--top-k must be from 1 to 32\n{QUERY_EXAMPLE}"
                    )));
                }
            };
            let response = llama::answer_with_files(
                &bundle,
                &chat_model,
                embed_model.as_deref(),
                &question,
                top_k,
                llama::query_reserve(),
                &wh_ask::default_config_dir(),
            )?;
            render_output(&response, &output)
        }
    }
}
