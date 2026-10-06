//! What each tool accepts. The doc comments on these types and fields become
//! the descriptions an agent reads, so they are written for the agent.

use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

/// Inputs for `search`.
#[derive(Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Words to look for in names and rules text, such as "lone operative".
    pub query: String,
    /// Only return these node kinds: Datasheet, Ability, Stratagem, Enhancement,
    /// Detachment, DetachmentAbility, Keyword, Faction, Wargear, Model. Omit to
    /// search every kind.
    pub kinds: Option<Vec<String>>,
    /// Most results to return, 1 to 50. Default 10.
    pub limit: Option<usize>,
}

/// Inputs for `find_units`.
#[derive(Deserialize, JsonSchema)]
pub struct FindUnitsParams {
    /// A unit name, such as "Custodian Guard".
    pub name: String,
}

/// Inputs for tools that take one faction.
#[derive(Deserialize, JsonSchema)]
pub struct FactionParams {
    /// A faction name such as "Adeptus Custodes", or a faction id. Use
    /// `list_factions` for the names.
    pub faction: String,
}

/// Inputs for `get_roster`.
#[derive(Deserialize, JsonSchema)]
pub struct RosterParams {
    /// A faction ("Space Marines"), a chapter ("Ultramarines", "Death Guard"), or
    /// a Legiones Daemonica god ("Khorne", "Tzeentch", "Nurgle", "Slaanesh"), by
    /// name or id.
    pub subject: String,
}

/// Parts of a unit that `get_unit` can return.
#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitSection {
    /// The datasheet's own text: loadout, transport capacity, leader rules, and
    /// damaged profile.
    Text,
    /// Keywords, marking the faction keywords.
    Keywords,
    /// Move, Toughness, Save, invulnerable save, Wounds, Leadership, and
    /// Objective Control per model.
    Models,
    /// How many models the unit has.
    Composition,
    /// The printed points lines.
    Points,
    /// The unit's own abilities with rules text.
    Abilities,
    /// Every weapon profile with its stats and keywords.
    Wargear,
    /// The wargear options.
    Options,
    /// Units this one can lead.
    Leads,
    /// Leaders that can join this unit.
    LedBy,
    /// Enhancements the unit can be given, across all detachments. Not in the default.
    Enhancements,
    /// Detachment rules that name this unit. Not in the default.
    DetachmentRules,
}

impl UnitSection {
    /// What `get_unit` returns when no sections are given: the full datasheet.
    pub const CARD: &'static [UnitSection] = &[
        UnitSection::Text,
        UnitSection::Keywords,
        UnitSection::Models,
        UnitSection::Composition,
        UnitSection::Points,
        UnitSection::Abilities,
        UnitSection::Wargear,
        UnitSection::Options,
        UnitSection::Leads,
        UnitSection::LedBy,
    ];
}

/// Inputs for `get_unit`.
#[derive(Deserialize, JsonSchema)]
pub struct UnitParams {
    /// A unit name such as "Angron", or a datasheet id (10ed:datasheet:...).
    pub unit: String,
    /// Which parts to return. Omit for the full datasheet (everything except
    /// enhancements and detachment_rules).
    pub sections: Option<Vec<UnitSection>>,
}

/// Parts of a detachment that `get_detachment` can return.
#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetachmentSection {
    /// The detachment's stratagems.
    Stratagems,
    /// The detachment's own rules.
    Rules,
    /// The detachment's enhancements.
    Enhancements,
}

impl DetachmentSection {
    /// What `get_detachment` returns when no sections are given.
    pub const ALL: &'static [DetachmentSection] = &[
        DetachmentSection::Stratagems,
        DetachmentSection::Rules,
        DetachmentSection::Enhancements,
    ];
}

/// Inputs for `get_detachment`.
#[derive(Deserialize, JsonSchema)]
pub struct DetachmentParams {
    /// A detachment name such as "Shield Host", or a detachment id. A name that
    /// several detachments share is an error listing their ids.
    pub detachment: String,
    /// Which parts to return. Omit for stratagems, rules, and enhancements.
    pub sections: Option<Vec<DetachmentSection>>,
}

/// Parts of a faction that `get_faction_rules` can return.
#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactionSection {
    /// The faction's army-wide abilities.
    Abilities,
    /// Every stratagem across the faction's detachments. A long list.
    Stratagems,
    /// Every enhancement across the faction's detachments. A long list.
    Enhancements,
}

impl FactionSection {
    /// What `get_faction_rules` returns when no sections are given.
    pub const DEFAULT: &'static [FactionSection] = &[FactionSection::Abilities];
}

/// Inputs for `get_faction_rules`.
#[derive(Deserialize, JsonSchema)]
pub struct FactionRulesParams {
    /// A faction name or id.
    pub faction: String,
    /// Which parts to return. Omit for abilities only, because stratagems and
    /// enhancements are long.
    pub sections: Option<Vec<FactionSection>>,
}

/// Inputs for `units_with_keyword`.
#[derive(Deserialize, JsonSchema)]
pub struct KeywordParams {
    /// A keyword such as "Infantry", "Psyker", or "Fly".
    pub keyword: String,
    /// Most units to return, 1 to 1000. Default 200. The reply gives the total.
    pub limit: Option<usize>,
}

/// Inputs for `units_with_ability`.
#[derive(Deserialize, JsonSchema)]
pub struct AbilityParams {
    /// An ability name such as "Feel No Pain" or "Deadly Demise".
    pub ability: String,
    /// Most units to return, 1 to 1000. Default 200. The reply gives the total.
    pub limit: Option<usize>,
}

/// What `units_for_rule` looks up.
#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    /// An enhancement: the units it can be given to.
    Enhancement,
    /// A detachment rule: the units it names.
    DetachmentRule,
}

/// Inputs for `units_for_rule`.
#[derive(Deserialize, JsonSchema)]
pub struct RuleUnitsParams {
    /// Whether `name` is an enhancement or a detachment rule.
    pub kind: RuleKind,
    /// The enhancement or rule name, such as "Idols of Khorne", or its id.
    pub name: String,
}

/// Inputs for `get_node`.
#[derive(Deserialize, JsonSchema)]
pub struct NodeParams {
    /// A node id returned by another tool, such as 10ed:datasheet:000002621.
    pub id: String,
}

/// Inputs for `get_neighbors`.
#[derive(Deserialize, JsonSchema)]
pub struct NeighborsParams {
    /// The node id to start from.
    pub id: String,
    /// Only follow these edge kinds, such as DATASHEET_HAS_KEYWORD. `bundle_info`
    /// lists the kinds. Omit for all kinds, which is large for keywords and factions.
    pub edge_kinds: Option<Vec<String>>,
    /// Most neighbors to return, 1 to 200. Default 50.
    pub limit: Option<usize>,
}

/// Inputs for `get_subgraph`.
#[derive(Deserialize, JsonSchema)]
pub struct SubgraphParams {
    /// One or more node ids to start from.
    pub seeds: Vec<String>,
    /// Only follow these edge kinds. `bundle_info` lists the kinds. Omit for all.
    pub edge_kinds: Option<Vec<String>>,
    /// How many edges out, 1 or 2. Default 1. Two edges out through a shared
    /// ability reaches every unit that has it.
    pub depth: Option<u32>,
}
