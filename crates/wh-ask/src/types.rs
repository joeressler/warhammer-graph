//! Serializable results the library returns. Nothing here holds prose written
//! by a model: every field is copied or parsed from the bundle.

use serde::Serialize;
use wh_graph::{Attrs, EdgeRecord};

/// A node by id: enough to name it and link to its source page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NodeRef {
    /// The node id, such as `10ed:faction:SM`. Pass it to any call that takes an id.
    pub id: String,
    /// The node kind, such as `Faction`, `Datasheet`, or `Stratagem`.
    pub kind: String,
    /// The node's label.
    pub name: String,
    /// The node's Wahapedia page, when it has one.
    pub wahapedia_link: Option<String>,
}

/// A whole node: its text and attributes as well as its name.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NodeDetail {
    /// The node id.
    pub id: String,
    /// The node kind.
    pub kind: String,
    /// The node's label.
    pub name: String,
    /// The node's plain text, as the bundle stores it.
    pub text: String,
    /// The node's attributes, such as a model's `M` and `T`.
    pub attrs: Attrs,
    /// The node's Wahapedia page, when it has one.
    pub wahapedia_link: Option<String>,
}

/// One label-search result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchHit {
    /// The matching node.
    pub node: NodeRef,
    /// The match score. Higher is better, and scores only compare within one search.
    pub score: u32,
}

/// A capped list. `total` counts everything that matched before the cap.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Page<T> {
    /// How many rows matched, including any beyond the cap.
    pub total: usize,
    /// The rows returned, at most the requested limit.
    pub items: Vec<T>,
}

/// Which end of an edge the queried node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// The queried node is the edge's `from`.
    Outgoing,
    /// The queried node is the edge's `to`.
    Incoming,
}

/// A node one edge away, with the edge that reaches it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Neighbor {
    /// The edge kind, such as `DATASHEET_HAS_KEYWORD`.
    pub edge_kind: String,
    /// Whether the queried node is the edge's `from` or its `to`.
    pub direction: Direction,
    /// The edge's attributes, such as an ability's `parameter`.
    pub edge_attrs: Attrs,
    /// The node at the other end.
    pub node: NodeRef,
}

/// The nodes and edges around a set of seeds.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Subgraph {
    /// The seeds, then every other node an edge reaches, each once.
    pub nodes: Vec<NodeRef>,
    /// The edges among the reached nodes, in bundle order.
    pub edges: Vec<EdgeRecord>,
    /// True when `edges` was cut at the limit.
    pub truncated: bool,
}

/// A datasheet by id: enough to name it, place it, and link to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnitRef {
    /// The datasheet id.
    pub id: String,
    /// The unit's name.
    pub name: String,
    /// The faction the datasheet belongs to.
    pub faction: Option<String>,
    /// The battlefield role, such as `Battleline` or `Characters`.
    pub role: Option<String>,
    /// The datasheet's Wahapedia page.
    pub wahapedia_link: Option<String>,
}

/// A faction, chapter, or daemon roster. `subject` is the faction or keyword the
/// name resolved to. Chapter and Legiones Daemonica rosters include the generic
/// parent units those armies can field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Roster {
    /// The faction or keyword the roster is for.
    pub subject: NodeRef,
    /// Every datasheet the subject can field.
    pub units: Vec<UnitRef>,
}

/// A keyword on a datasheet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Keyword {
    /// The keyword node id.
    pub id: String,
    /// The keyword as printed.
    pub name: String,
    /// True for a faction keyword, such as `Adeptus Astartes`.
    pub is_faction_keyword: bool,
}

/// One row of a model profile. Characteristics are the printed values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelProfile {
    /// The model node id.
    pub id: String,
    /// The model's name.
    pub name: String,
    /// Move, such as `6"`.
    pub movement: String,
    /// Toughness.
    pub toughness: String,
    /// Armour save, such as `3+`.
    pub save: String,
    /// The invulnerable save, written like `4+`. `None` when the model has none.
    pub invulnerable_save: Option<String>,
    /// A condition on the invulnerable save, such as "Against ranged attacks only".
    pub invulnerable_save_note: Option<String>,
    /// Wounds.
    pub wounds: String,
    /// Leadership, such as `6+`.
    pub leadership: String,
    /// Objective Control.
    pub objective_control: String,
}

/// One printed points line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PointsCost {
    /// The cost node id.
    pub id: String,
    /// What the cost covers, such as `5 models`.
    pub description: String,
    /// The points. `None` when the printed cost is not a whole number.
    pub points: Option<u32>,
}

/// One line of a unit's composition, such as `1 Warboss` or `5-10 Boyz`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnitComposition {
    /// The composition node id.
    pub id: String,
    /// The line as printed.
    pub text: String,
}

/// One wargear option, such as "This model's big choppa can be replaced with 1 power klaw."
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WargearOption {
    /// The option node id.
    pub id: String,
    /// The option as printed, without its leading bullet.
    pub text: String,
}

/// An ability a datasheet has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ability {
    /// The ability node id.
    pub id: String,
    /// The ability's name.
    pub name: String,
    /// The export's ability type for this unit: Core, Faction, Datasheet, Wargear, and so on.
    pub scope: Option<String>,
    /// A per-unit value for a shared ability, such as the 5+ in Feel No Pain 5+.
    pub parameter: Option<String>,
    /// The model this ability is limited to, when the sheet names one.
    pub model: Option<String>,
    /// The rules text, without the name line.
    pub text: String,
}

/// The numbers on a weapon profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WeaponStats {
    /// Range in inches, or `Melee`.
    pub range: String,
    /// `Ranged` or `Melee`.
    pub kind: String,
    /// Attacks, such as `2` or `D6+1`.
    pub attacks: String,
    /// Ballistic Skill for a ranged weapon, Weapon Skill for a melee one.
    pub skill: String,
    /// Strength.
    pub strength: String,
    /// Armour Penetration, such as `-1`.
    pub ap: String,
    /// Damage.
    pub damage: String,
}

/// One weapon profile. A weapon with two profiles, such as strike and sweep,
/// is two entries whose names say which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Weapon {
    /// The wargear node id.
    pub id: String,
    /// The profile's name, such as `Gork’s Klaw - strike`.
    pub name: String,
    /// `None` when the passage has no recognizable stat line; `text` still has it.
    pub stats: Option<WeaponStats>,
    /// Weapon keywords, such as `sustained hits 1`.
    pub keywords: Vec<String>,
    /// The whole passage as stored.
    pub text: String,
}

/// A stratagem, enhancement, or detachment rule: the name and its rules text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleText {
    /// The node id.
    pub id: String,
    /// The rule's name.
    pub name: String,
    /// The rules text, without the name line.
    pub text: String,
}

/// A unit that has an ability, with that unit's parameter for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AbilityHolder {
    /// The unit that has the ability.
    pub unit: UnitRef,
    /// The unit's value for the ability, such as `5+`.
    pub parameter: Option<String>,
    /// The model the ability is limited to, when the sheet names one.
    pub model: Option<String>,
}

/// Everything the bundle holds about one datasheet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnitCard {
    /// The datasheet.
    pub unit: UnitRef,
    /// The datasheet's own text: its loadout, transport capacity, leader rules,
    /// and damaged profile, as printed. Empty when it has none.
    pub text: String,
    /// The unit's keywords.
    pub keywords: Vec<Keyword>,
    /// One profile per model type.
    pub models: Vec<ModelProfile>,
    /// How many models the unit has.
    pub composition: Vec<UnitComposition>,
    /// The printed points lines.
    pub points: Vec<PointsCost>,
    /// The unit's own abilities.
    pub abilities: Vec<Ability>,
    /// Every weapon profile.
    pub wargear: Vec<Weapon>,
    /// The wargear options.
    pub options: Vec<WargearOption>,
    /// Units this one can lead.
    pub leads: Vec<UnitRef>,
    /// Leaders that can join this unit.
    pub led_by: Vec<UnitRef>,
}
