use std::fmt;
use std::path::Path;

/// Failures mapped onto the `query` exit codes.
#[derive(Debug)]
pub enum AskError {
    /// A required flag is missing or `--top-k` is outside 1..=32. Exit 2.
    Usage(String),
    /// The bundle is unreadable or `format_version` is not 1. Exit 3.
    Bundle(String),
    /// A GGUF failed to load, or retrieval could not finish. Exit 4.
    Model(String),
}

impl AskError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AskError::Usage(_) => 2,
            AskError::Bundle(_) => 3,
            AskError::Model(_) => 4,
        }
    }

    pub fn usage(detail: impl fmt::Display) -> Self {
        AskError::Usage(detail.to_string())
    }

    pub fn bundle(path: &Path, detail: impl fmt::Display) -> Self {
        AskError::Bundle(format!("{}: {detail}", path.display()))
    }

    pub fn model(detail: impl fmt::Display) -> Self {
        AskError::Model(detail.to_string())
    }
}

impl fmt::Display for AskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AskError::Usage(message) | AskError::Bundle(message) | AskError::Model(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for AskError {}
