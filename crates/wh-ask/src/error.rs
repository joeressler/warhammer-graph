use std::fmt;
use std::path::Path;

use serde::Serialize;

/// One of several nodes a name could mean.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// The node id to pass instead of the name.
    pub id: String,
    /// The node's label.
    pub name: String,
    /// What tells it apart from the others: a unit's or detachment's faction, or
    /// the detachment that owns an enhancement, stratagem, or rule. Empty when
    /// nothing does.
    pub detail: String,
}

/// Why a call failed. The messages say what to do next.
#[derive(Debug)]
pub enum AskError {
    /// The bundle is unreadable or `format_version` is not 2.
    Bundle(String),
    /// Nothing carries that name or id.
    NotFound {
        /// What was looked for, such as `Datasheet named "Taxtical Squad"`.
        what: String,
        /// Close names to try. For a roster it is the faction names, and for an
        /// edge kind it is the known kinds.
        suggestions: Vec<String>,
    },
    /// A name matches several nodes. Pass one of the candidate ids instead.
    Ambiguous {
        /// What was looked for.
        what: String,
        /// Every node the name matches.
        candidates: Vec<Candidate>,
    },
    /// An id or argument is the wrong kind for the call.
    Invalid(String),
}

impl AskError {
    pub(crate) fn bundle(path: &Path, detail: impl fmt::Display) -> Self {
        AskError::Bundle(format!("{}: {detail}", path.display()))
    }

    pub(crate) fn invalid(detail: impl fmt::Display) -> Self {
        AskError::Invalid(detail.to_string())
    }

    pub(crate) fn not_found(what: impl fmt::Display, suggestions: Vec<String>) -> Self {
        AskError::NotFound {
            what: what.to_string(),
            suggestions,
        }
    }
}

impl fmt::Display for AskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AskError::Bundle(message) | AskError::Invalid(message) => f.write_str(message),
            AskError::NotFound { what, suggestions } => {
                write!(f, "no {what} found")?;
                if !suggestions.is_empty() {
                    write!(f, "; did you mean: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            AskError::Ambiguous { what, candidates } => {
                write!(f, "{what} matches {} nodes; pass an id instead: ", candidates.len())?;
                let shown = candidates
                    .iter()
                    .map(|item| {
                        if item.detail.is_empty() {
                            format!("{} ({})", item.id, item.name)
                        } else {
                            format!("{} ({}, {})", item.id, item.name, item.detail)
                        }
                    })
                    .collect::<Vec<_>>();
                f.write_str(&shown.join("; "))
            }
        }
    }
}

impl std::error::Error for AskError {}
