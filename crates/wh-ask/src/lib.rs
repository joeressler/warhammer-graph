//! Read-only access to a Wahapedia graph bundle: name lookup, passage search,
//! graph navigation, and typed lists (rosters, abilities, wargear, points,
//! model profiles, leaders, stratagems, enhancements).
//!
//! Nothing here generates text. A caller opens a [`Bundle`], asks for what it
//! needs, and gets structured rows back. Every result type implements
//! `serde::Serialize`, so it can be returned as JSON unchanged.
//!
//! ```no_run
//! use std::path::Path;
//! use wh_ask::{AskError, Bundle};
//!
//! # fn main() -> Result<(), AskError> {
//! let bundle = Bundle::open(Path::new("./bundle"))?;
//!
//! // A faction, chapter, or daemon god: every unit it can field.
//! let roster = bundle.roster("Ultramarines")?;
//! println!("{} units", roster.units.len());
//!
//! // One call for everything about a datasheet.
//! let card = bundle.unit("Angron")?;
//! println!("{:?}", card.models[0].invulnerable_save);
//!
//! // A name two datasheets share is an error that lists each id.
//! match bundle.unit("Dreadnought") {
//!     Err(AskError::Ambiguous { candidates, .. }) => {
//!         let first = &candidates[0].id;
//!         let _ = bundle.unit(first)?;
//!     }
//!     _ => {}
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Where to look
//!
//! - [`Bundle::open`] opens a bundle. Everything else is a method on [`Bundle`].
//! - Find: [`Bundle::search`], [`Bundle::find_units`].
//! - Navigate: [`Bundle::node_detail`], [`Bundle::neighbors`], [`Bundle::subgraph`].
//! - Lists: [`Bundle::roster`], [`Bundle::unit`], and the `unit_*`, `detachment_*`,
//!   `faction_*`, and `units_with_*` methods.
//!
//! Calls that take a unit, faction, detachment, keyword, ability, or enhancement
//! accept a node id or an exact name. A name that matches nothing is
//! [`AskError::NotFound`] with suggestions. A name that matches several nodes is
//! [`AskError::Ambiguous`] with each candidate's id.
#![warn(missing_docs)]

mod bundle;
mod error;
mod graph;
mod lists;
mod names;
mod roster;
mod search;
mod types;

pub use bundle::Bundle;
pub use error::{AskError, Candidate};
pub use types::{
    Ability, AbilityHolder, Direction, Enhancement, Keyword, ModelProfile, Neighbor, NodeDetail, NodeRef,
    Page, PointsCost, Roster, RuleText, SearchHit, Stratagem, Subgraph, UnitCard, UnitComposition, UnitRef,
    WargearOption, Weapon, WeaponStats,
};
pub use wh_graph::{Attrs, BundleManifest, EdgeRecord};
