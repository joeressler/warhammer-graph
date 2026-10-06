//! `wh-mcp`: serve a Wahapedia graph bundle to an MCP client over stdio.
//!
//! Stdout carries the protocol, so nothing else may be written to it while the
//! server runs. Messages meant for a person go to stderr.

mod params;
mod reply;
mod server;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use rmcp::transport::stdio;
use rmcp::ServiceExt;
use wh_ask::Bundle;

use crate::server::WhServer;

const USAGE: &str = "\
wh-mcp: serve a Wahapedia graph bundle to an MCP client over stdio.

Usage:
  wh-mcp --bundle ./bundle
  WH_BUNDLE=./bundle wh-mcp

Build the bundle first with:
  wh-graph build --corpus ./corpus --out ./bundle

Example client entry (Claude Code):
  claude mcp add wh -- wh-mcp --bundle ./bundle
";

/// What the command line asked for.
enum Command {
    Help,
    Version,
    Serve(PathBuf),
}

#[tokio::main]
async fn main() -> ExitCode {
    let path = match parse(env::args().skip(1), env::var("WH_BUNDLE").ok()) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            println!("wh-mcp {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Command::Serve(path)) => path,
        Err(message) => {
            eprintln!("wh-mcp: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let bundle = match Bundle::open(&path) {
        Ok(bundle) => Arc::new(bundle),
        Err(error) => {
            eprintln!("wh-mcp: {error}");
            return ExitCode::from(3);
        }
    };
    eprintln!(
        "wh-mcp: {} nodes and {} edges loaded from {}",
        bundle.manifest().node_count,
        bundle.manifest().edge_count,
        path.display()
    );

    let running = match WhServer::new(bundle).serve(stdio()).await {
        Ok(running) => running,
        Err(error) => {
            eprintln!("wh-mcp: could not start the MCP session: {error}");
            return ExitCode::from(1);
        }
    };
    match running.waiting().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wh-mcp: {error}");
            ExitCode::from(1)
        }
    }
}

/// Read the command line. `from_env` is `WH_BUNDLE`, used when `--bundle` is absent.
fn parse(args: impl Iterator<Item = String>, from_env: Option<String>) -> Result<Command, String> {
    let mut bundle = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--version" | "-V" => return Ok(Command::Version),
            "--bundle" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--bundle needs a directory".to_string())?;
                bundle = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    bundle
        .or_else(|| from_env.filter(|value| !value.is_empty()).map(PathBuf::from))
        .map(Command::Serve)
        .ok_or_else(|| "no bundle given; pass --bundle or set WH_BUNDLE".to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse, Command};
    use std::path::PathBuf;

    fn run(args: &[&str], env: Option<&str>) -> Result<Command, String> {
        parse(args.iter().map(|arg| arg.to_string()), env.map(str::to_string))
    }

    #[test]
    fn the_flag_wins_over_the_environment() {
        match run(&["--bundle", "./a"], Some("./b")) {
            Ok(Command::Serve(path)) => assert_eq!(path, PathBuf::from("./a")),
            _ => panic!("expected Serve"),
        }
        match run(&[], Some("./b")) {
            Ok(Command::Serve(path)) => assert_eq!(path, PathBuf::from("./b")),
            _ => panic!("expected Serve from the environment"),
        }
    }

    #[test]
    fn a_missing_bundle_or_a_bad_flag_is_an_error() {
        assert!(run(&[], None).is_err());
        assert!(run(&[], Some("")).is_err());
        assert!(run(&["--bundle"], None).is_err());
        assert!(run(&["--nope"], None).is_err());
        assert!(matches!(run(&["--help"], None), Ok(Command::Help)));
        assert!(matches!(run(&["--version"], None), Ok(Command::Version)));
    }
}
