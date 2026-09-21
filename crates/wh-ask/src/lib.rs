//! Load a petgraph bundle and answer a question from its passages.

mod bundle;
mod cite;
mod error;
mod expand;
mod index;
mod model;
mod prompt;
mod query;
mod retrieve;
mod settings;

pub use bundle::load_bundle;
pub use cite::Citation;
pub use error::AskError;
pub use index::model_identity;
pub use model::{ChatModel, EmbedIdentity, Embedder};
pub use query::{render_output, run_query, QueryRequest, QueryResponse, Retrieval};
pub use settings::{
    absolute_path, default_config_dir, load_settings, save_settings, Settings, DEFAULT_RESERVE,
    DEFAULT_TOP_K,
};

/// One working `query` invocation, printed with usage errors.
pub const QUERY_EXAMPLE: &str = "wh-ask query --bundle ./bundle --chat-model ./models/chat.gguf --question \"Which example units are in Example Faction?\"";
