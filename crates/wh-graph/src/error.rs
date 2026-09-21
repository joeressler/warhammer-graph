use std::fmt;
use std::path::Path;

/// Failures mapped onto the spec's exit codes.
#[derive(Debug)]
pub enum GraphError {
    /// Missing flag or a bad `--output`. Exit 2.
    Usage(String),
    /// The corpus or bundle cannot be read, or a version is not 1. Exit 3.
    Read(String),
    /// A graph rule failed. The message names a node id. Exit 4.
    Invalid(String),
}

impl GraphError {
    pub fn exit_code(&self) -> i32 {
        match self {
            GraphError::Usage(_) => 2,
            GraphError::Read(_) => 3,
            GraphError::Invalid(_) => 4,
        }
    }

    pub fn read(path: &Path, detail: impl fmt::Display) -> Self {
        GraphError::Read(format!("{}: {detail}", path.display()))
    }

    pub fn invalid(node_id: &str, rule: impl fmt::Display) -> Self {
        GraphError::Invalid(format!("{node_id}: {rule}"))
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::Usage(message)
            | GraphError::Read(message)
            | GraphError::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for GraphError {}
