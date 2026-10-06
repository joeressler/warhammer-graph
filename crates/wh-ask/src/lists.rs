//! Typed, deterministic lists: rosters, abilities, wargear, points, model
//! profiles, leaders, and rules. Each call reads edges and nodes from the
//! bundle and returns every matching row, never a ranked subset.
//!
//! Unit arguments take a datasheet id or an exact name. A name shared by
//! several datasheets gives [`AskError::Ambiguous`] listing each id and faction.

use std::collections::HashSet;

use wh_graph::{EdgeRecord, GraphNode};

use crate::bundle::Bundle;
use crate::error::AskError;
use crate::roster;
use crate::types::{
    Ability, AbilityHolder, Keyword, ModelProfile, NodeRef, PointsCost, Roster, RuleText,
    UnitCard, UnitComposition, UnitRef, WargearOption, Weapon, WeaponStats,
};

impl Bundle {
    /// Every faction, by name.
    pub fn factions(&self) -> Vec<NodeRef> {
        let mut factions = self
            .nodes_of("Faction")
            .map(|node| self.node_ref(node))
            .collect::<Vec<_>>();
        factions.sort_by(|left, right| left.name.cmp(&right.name));
        factions
    }

    /// A faction's detachments, by name.
    pub fn detachments(&self, faction: &str) -> Result<Vec<NodeRef>, AskError> {
        let id = self.resolve("Faction", faction)?;
        let mut found = self
            .linked_from(&id, "FACTION_HAS_DETACHMENT")?
            .into_iter()
            .map(|(_, node)| self.node_ref(node))
            .collect::<Vec<_>>();
        found.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(found)
    }

    /// Datasheets with this exact name in any faction, or the closest names
    /// when none match exactly. An id returns that one unit.
    pub fn find_units(&self, name: &str) -> Vec<UnitRef> {
        let name = name.trim();
        if let Some(unit) = self.unit_ref(name) {
            return vec![unit];
        }
        let exact = self
            .ids_named("Datasheet", name)
            .iter()
            .filter_map(|id| self.unit_ref(id))
            .collect::<Vec<_>>();
        if !exact.is_empty() {
            return exact;
        }
        self.search(name, &["Datasheet"], 10)
            .into_iter()
            .filter_map(|hit| self.unit_ref(&hit.node.id))
            .collect()
    }

    /// The full unit list for a faction, a chapter, or a Legiones Daemonica god.
    /// Chapters and daemon gods include the generic parent units they can field.
    pub fn roster(&self, subject: &str) -> Result<Roster, AskError> {
        let subject = subject.trim();
        let name = match self.node(subject) {
            Some(node) if matches!(node.kind.as_str(), "Faction" | "Keyword") => {
                node.label.clone()
            }
            Some(node) => {
                return Err(AskError::invalid(format!(
                    "{subject} is a {}, not a faction or chapter",
                    node.kind
                )));
            }
            None => subject.to_string(),
        };
        let Some(found) = roster::roster(self, &name) else {
            return Err(AskError::not_found(
                format!("faction, chapter, or daemon god \"{subject}\""),
                self.factions().into_iter().map(|faction| faction.name).collect(),
            ));
        };
        let subject_node = self
            .node(&found.subject)
            .map(|node| self.node_ref(node))
            .ok_or_else(|| AskError::invalid("roster subject is not in the bundle"))?;
        let units = found
            .datasheets
            .iter()
            .filter_map(|id| self.unit_ref(id))
            .collect();
        Ok(Roster {
            subject: subject_node,
            units,
        })
    }

    /// Every unit carrying a keyword (Infantry, Psyker, and so on).
    pub fn units_with_keyword(&self, keyword: &str) -> Result<Vec<UnitRef>, AskError> {
        let id = self.resolve("Keyword", keyword)?;
        let mut seen = HashSet::new();
        let mut units = Vec::new();
        for edge in self.edges_at(&id, "DATASHEET_HAS_KEYWORD") {
            if edge.to == id && seen.insert(edge.from.clone()) {
                units.extend(self.unit_ref(&edge.from));
            }
        }
        Ok(units)
    }

    /// Every unit with an ability, and that unit's parameter for it. Several
    /// ability nodes can share a name (a core rule and a datasheet's own copy);
    /// all are included.
    pub fn units_with_ability(&self, ability: &str) -> Result<Vec<AbilityHolder>, AskError> {
        let ids = self.resolve_all("Ability", ability)?;
        let wanted = ids.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        let mut holders = Vec::new();
        for edge in self.expand_edges(&ids, &["DATASHEET_HAS_ABILITY"], 1) {
            if !wanted.contains(edge.to.as_str()) {
                continue;
            }
            let parameter = non_empty(edge.attrs.text("parameter"));
            let model = non_empty(edge.attrs.text("model"));
            let key = (edge.from.clone(), parameter.clone(), model.clone());
            if !seen.insert(key) {
                continue;
            }
            if let Some(unit) = self.unit_ref(&edge.from) {
                holders.push(AbilityHolder {
                    unit,
                    parameter,
                    model,
                });
            }
        }
        Ok(holders)
    }

    /// One call for everything about a datasheet.
    pub fn unit(&self, unit: &str) -> Result<UnitCard, AskError> {
        let id = self.resolve("Datasheet", unit)?;
        Ok(UnitCard {
            unit: self
                .unit_ref(&id)
                .ok_or_else(|| AskError::invalid(format!("{id} is not a datasheet")))?,
            text: self.text_of(&id),
            keywords: self.keywords_of(&id)?,
            models: self.models_of(&id)?,
            composition: self.composition_of(&id)?,
            points: self.points_of(&id)?,
            abilities: self.abilities_of(&id)?,
            wargear: self.wargear_of(&id)?,
            options: self.options_of(&id)?,
            leads: self.lead_units(&id, true)?,
            led_by: self.lead_units(&id, false)?,
        })
    }

    /// The datasheet's keywords, marking the faction keywords.
    pub fn unit_keywords(&self, unit: &str) -> Result<Vec<Keyword>, AskError> {
        self.keywords_of(&self.resolve("Datasheet", unit)?)
    }

    /// The datasheet's own text: its loadout, transport capacity, leader rules,
    /// and damaged profile, as printed. Empty when it has none.
    pub fn unit_text(&self, unit: &str) -> Result<String, AskError> {
        Ok(self.text_of(&self.resolve("Datasheet", unit)?))
    }

    /// How many models the unit has, one line per entry, such as `1 Warboss` or `5-10 Boyz`.
    pub fn unit_composition(&self, unit: &str) -> Result<Vec<UnitComposition>, AskError> {
        self.composition_of(&self.resolve("Datasheet", unit)?)
    }

    /// The wargear options, such as swapping one weapon for another.
    pub fn unit_options(&self, unit: &str) -> Result<Vec<WargearOption>, AskError> {
        self.options_of(&self.resolve("Datasheet", unit)?)
    }

    /// Move, Toughness, Save, Wounds, Leadership, and Objective Control per model.
    pub fn unit_models(&self, unit: &str) -> Result<Vec<ModelProfile>, AskError> {
        self.models_of(&self.resolve("Datasheet", unit)?)
    }

    /// The printed points lines, such as "1 model — 75" or "5 models — 90".
    pub fn unit_points(&self, unit: &str) -> Result<Vec<PointsCost>, AskError> {
        self.points_of(&self.resolve("Datasheet", unit)?)
    }

    /// The unit's own abilities, with scope, parameter, and rules text.
    pub fn unit_abilities(&self, unit: &str) -> Result<Vec<Ability>, AskError> {
        self.abilities_of(&self.resolve("Datasheet", unit)?)
    }

    /// Every weapon profile, one entry each. A weapon with strike and sweep
    /// profiles is two entries.
    pub fn unit_wargear(&self, unit: &str) -> Result<Vec<Weapon>, AskError> {
        self.wargear_of(&self.resolve("Datasheet", unit)?)
    }

    /// Units this datasheet can lead (it is the leader).
    pub fn unit_leads(&self, unit: &str) -> Result<Vec<UnitRef>, AskError> {
        self.lead_units(&self.resolve("Datasheet", unit)?, true)
    }

    /// Leaders that can be attached to this datasheet.
    pub fn unit_led_by(&self, unit: &str) -> Result<Vec<UnitRef>, AskError> {
        self.lead_units(&self.resolve("Datasheet", unit)?, false)
    }

    /// Every enhancement that can be given to this unit, across all detachments.
    pub fn unit_enhancements(&self, unit: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Datasheet", unit)?;
        self.rules_into(&id, "ENHANCEMENT_APPLIES_TO_DATASHEET")
    }

    /// The detachment rules that name this unit.
    pub fn unit_detachment_rules(&self, unit: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Datasheet", unit)?;
        self.rules_from(&id, "DATASHEET_HAS_DETACHMENT_ABILITY")
    }

    /// The units an enhancement can be given to. A name shared by enhancements in
    /// several detachments is `Ambiguous`, and each candidate names its detachment.
    pub fn enhancement_units(&self, enhancement: &str) -> Result<Vec<UnitRef>, AskError> {
        let id = self.resolve("Enhancement", enhancement)?;
        Ok(self.units_linked(&id, "ENHANCEMENT_APPLIES_TO_DATASHEET", true))
    }

    /// The units a detachment rule names, such as who gets Idols of Khorne. A name
    /// shared by rules in several detachments is `Ambiguous`.
    pub fn detachment_rule_units(&self, rule: &str) -> Result<Vec<UnitRef>, AskError> {
        let id = self.resolve("DetachmentAbility", rule)?;
        Ok(self.units_linked(&id, "DATASHEET_HAS_DETACHMENT_ABILITY", false))
    }

    /// A faction's own abilities, such as an army-wide rule.
    pub fn faction_abilities(&self, faction: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Faction", faction)?;
        self.rules_from(&id, "FACTION_HAS_ABILITY")
    }

    /// Every stratagem a faction has across its detachments.
    pub fn faction_stratagems(&self, faction: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Faction", faction)?;
        self.rules_from(&id, "FACTION_HAS_STRATAGEM")
    }

    /// Every stratagem in a detachment.
    pub fn detachment_stratagems(&self, detachment: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Detachment", detachment)?;
        self.rules_from(&id, "DETACHMENT_HAS_STRATAGEM")
    }

    /// A detachment's own rules (its detachment abilities).
    pub fn detachment_rules(&self, detachment: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Detachment", detachment)?;
        self.rules_from(&id, "DETACHMENT_HAS_ABILITY")
    }

    /// Every enhancement a detachment offers.
    pub fn detachment_enhancements(&self, detachment: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Detachment", detachment)?;
        self.rules_from(&id, "DETACHMENT_HAS_ENHANCEMENT")
    }

    /// Every enhancement a faction has across its detachments.
    pub fn faction_enhancements(&self, faction: &str) -> Result<Vec<RuleText>, AskError> {
        let id = self.resolve("Faction", faction)?;
        self.rules_from(&id, "FACTION_HAS_ENHANCEMENT")
    }

    pub(crate) fn unit_ref(&self, id: &str) -> Option<UnitRef> {
        let node = self.node(id)?;
        if node.kind != "Datasheet" {
            return None;
        }
        Some(UnitRef {
            id: node.id.clone(),
            name: node.label.clone(),
            faction: node
                .attrs
                .text("faction_node")
                .and_then(|faction| self.node(faction))
                .map(|faction| faction.label.clone()),
            role: non_empty(node.attrs.text("role")),
            wahapedia_link: node.source_url.clone(),
        })
    }

    /// Edges of `kind` leaving `id`, with the node each one reaches.
    fn linked_from(
        &self,
        id: &str,
        kind: &str,
    ) -> Result<Vec<(&EdgeRecord, &GraphNode)>, AskError> {
        Ok(self
            .edges_at(id, kind)
            .into_iter()
            .filter(|edge| edge.from == id)
            .filter_map(|edge| {
                let node = self.node(&edge.to)?;
                Some((edge, node))
            })
            .collect())
    }

    fn keywords_of(&self, id: &str) -> Result<Vec<Keyword>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_KEYWORD")?
            .into_iter()
            .map(|(edge, node)| Keyword {
                id: node.id.clone(),
                name: node.label.clone(),
                is_faction_keyword: edge.attrs.text("is_faction_keyword") == Some("true"),
            })
            .collect())
    }

    fn models_of(&self, id: &str) -> Result<Vec<ModelProfile>, AskError> {
        let text = |node: &GraphNode, key: &str| node.attrs.text(key).unwrap_or("").to_string();
        Ok(self
            .linked_from(id, "DATASHEET_HAS_MODEL")?
            .into_iter()
            .map(|(_, node)| {
                let invulnerable_save = invulnerable_save(node.attrs.text("inv_sv"));
                ModelProfile {
                    id: node.id.clone(),
                    name: node.label.clone(),
                    movement: text(node, "M"),
                    toughness: text(node, "T"),
                    save: text(node, "Sv"),
                    invulnerable_save_note: invulnerable_save
                        .as_ref()
                        .and_then(|_| invulnerable_note(node.attrs.text("inv_sv_descr"))),
                    invulnerable_save,
                    wounds: text(node, "W"),
                    leadership: text(node, "Ld"),
                    objective_control: text(node, "OC"),
                }
            })
            .collect())
    }

    fn points_of(&self, id: &str) -> Result<Vec<PointsCost>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_COST")?
            .into_iter()
            .map(|(_, node)| points_cost(node))
            .collect())
    }

    fn abilities_of(&self, id: &str) -> Result<Vec<Ability>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_ABILITY")?
            .into_iter()
            .map(|(edge, node)| {
                let scope = non_empty(edge.attrs.text("type"));
                Ability {
                    id: node.id.clone(),
                    name: node.label.clone(),
                    parameter: non_empty(edge.attrs.text("parameter")),
                    model: non_empty(edge.attrs.text("model")),
                    text: rules_text(node, scope.as_deref()),
                    scope,
                }
            })
            .collect())
    }

    fn wargear_of(&self, id: &str) -> Result<Vec<Weapon>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_WARGEAR")?
            .into_iter()
            .map(|(_, node)| weapon(node))
            .collect())
    }

    /// Datasheets across `DATASHEET_CAN_LEAD`: the ones `id` leads when
    /// `outgoing`, otherwise the leaders that lead `id`.
    fn lead_units(&self, id: &str, outgoing: bool) -> Result<Vec<UnitRef>, AskError> {
        let mut seen = HashSet::new();
        let mut units = Vec::new();
        for edge in self.edges_at(id, "DATASHEET_CAN_LEAD") {
            let other = if outgoing && edge.from == id {
                &edge.to
            } else if !outgoing && edge.to == id {
                &edge.from
            } else {
                continue;
            };
            if other != id && seen.insert(other.clone()) {
                units.extend(self.unit_ref(other));
            }
        }
        units.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(units)
    }

    fn rules_from(&self, id: &str, kind: &str) -> Result<Vec<RuleText>, AskError> {
        Ok(self
            .linked_from(id, kind)?
            .into_iter()
            .map(|(_, node)| RuleText {
                id: node.id.clone(),
                name: node.label.clone(),
                text: rules_text(node, None),
            })
            .collect())
    }

    /// Like `rules_from`, for edges that arrive at `id`.
    fn rules_into(&self, id: &str, kind: &str) -> Result<Vec<RuleText>, AskError> {
        Ok(self
            .edges_at(id, kind)
            .into_iter()
            .filter(|edge| edge.to == id)
            .filter_map(|edge| self.node(&edge.from))
            .map(|node| RuleText {
                id: node.id.clone(),
                name: node.label.clone(),
                text: rules_text(node, None),
            })
            .collect())
    }

    /// Datasheets across an edge of `kind`: the ones it points to when `id_is_from`,
    /// otherwise the ones that point at `id`. Each unit appears once.
    fn units_linked(&self, id: &str, kind: &str, id_is_from: bool) -> Vec<UnitRef> {
        let mut seen = HashSet::new();
        let mut units = Vec::new();
        for edge in self.edges_at(id, kind) {
            let other = if id_is_from && edge.from == id {
                &edge.to
            } else if !id_is_from && edge.to == id {
                &edge.from
            } else {
                continue;
            };
            if seen.insert(other.clone()) {
                units.extend(self.unit_ref(other));
            }
        }
        units
    }

    fn text_of(&self, id: &str) -> String {
        self.node(id)
            .map(|node| rules_text(node, node.attrs.text("role")))
            .unwrap_or_default()
    }

    fn composition_of(&self, id: &str) -> Result<Vec<UnitComposition>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_COMPOSITION")?
            .into_iter()
            .map(|(_, node)| UnitComposition {
                id: node.id.clone(),
                text: node.text.trim().to_string(),
            })
            .collect())
    }

    fn options_of(&self, id: &str) -> Result<Vec<WargearOption>, AskError> {
        Ok(self
            .linked_from(id, "DATASHEET_HAS_OPTION")?
            .into_iter()
            .map(|(_, node)| WargearOption {
                id: node.id.clone(),
                text: node.text.trim().trim_start_matches('•').trim().to_string(),
            })
            .collect())
    }
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// The export prints an invulnerable save as `4`, `5`, or `4*`, and `-` for none.
/// This returns `4+`, or `None` for no save.
fn invulnerable_save(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().trim_end_matches('*').trim();
    if value.is_empty() || value == "-" {
        return None;
    }
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(format!("{value}+"));
    }
    Some(value.to_string())
}

/// The condition text without the leading `*` the export uses to mark it.
fn invulnerable_note(raw: Option<&str>) -> Option<String> {
    let note = raw?.trim().trim_start_matches('*').trim();
    (!note.is_empty()).then(|| note.to_string())
}

/// A node's text without its leading name line, and without the ability type
/// line the export repeats on datasheet abilities.
fn rules_text(node: &GraphNode, scope: Option<&str>) -> String {
    let mut lines = node.text.lines().map(str::trim).peekable();
    while lines.peek().is_some_and(|line| line.is_empty()) {
        lines.next();
    }
    if lines
        .peek()
        .is_some_and(|line| caseless::default_caseless_match_str(line, &node.label))
    {
        lines.next();
    }
    if let Some(scope) = scope {
        if lines
            .peek()
            .is_some_and(|line| caseless::default_caseless_match_str(line, scope))
        {
            lines.next();
        }
    }
    lines.collect::<Vec<_>>().join("\n").trim().to_string()
}

/// The cost passage is the description, then the points on the last line.
fn points_cost(node: &GraphNode) -> PointsCost {
    let mut lines = node
        .text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let cost = lines.pop().unwrap_or("");
    PointsCost {
        id: node.id.clone(),
        description: lines.join(", "),
        points: cost.parse().ok(),
    }
}

/// A weapon passage is its name, one stat line, then its keywords.
fn weapon(node: &GraphNode) -> Weapon {
    let lines = node
        .text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let at = lines.iter().position(|line| weapon_stats(line).is_some());
    let stats = at.and_then(|index| weapon_stats(lines[index]));
    let keywords = at
        .map(|index| {
            lines[index + 1..]
                .iter()
                .flat_map(|line| line.split(','))
                .map(str::trim)
                .filter(|keyword| !keyword.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Weapon {
        id: node.id.clone(),
        name: node.label.clone(),
        stats,
        keywords,
        text: node.text.clone(),
    }
}

/// `Range Type A Skill S AP D`, where Type is Ranged or Melee.
fn weapon_stats(line: &str) -> Option<WeaponStats> {
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.len() < 7
        || !(words[1].eq_ignore_ascii_case("ranged") || words[1].eq_ignore_ascii_case("melee"))
    {
        return None;
    }
    Some(WeaponStats {
        range: words[0].to_string(),
        kind: words[1].to_string(),
        attacks: words[2].to_string(),
        skill: words[3].to_string(),
        strength: words[4].to_string(),
        ap: words[5].to_string(),
        damage: words[6].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use wh_graph::{Attrs, GraphNode};

    use super::{invulnerable_note, invulnerable_save, points_cost, rules_text, weapon};

    fn node(id: &str, label: &str, text: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            kind: String::new(),
            label: label.to_string(),
            text: text.to_string(),
            attrs: Attrs::default(),
            source_url: None,
        }
    }

    #[test]
    fn a_ranged_weapon_has_stats_and_keywords() {
        let found = weapon(&node(
            "w",
            "Kombi-weapon",
            "Kombi-weapon\n24 Ranged 1 5 4 0 1\nanti-infantry 4+, devastating wounds, rapid fire 1",
        ));
        let stats = found.stats.unwrap();
        assert_eq!(
            (
                stats.range.as_str(),
                stats.kind.as_str(),
                stats.attacks.as_str(),
                stats.skill.as_str(),
                stats.strength.as_str(),
                stats.ap.as_str(),
                stats.damage.as_str()
            ),
            ("24", "Ranged", "1", "5", "4", "0", "1")
        );
        assert_eq!(
            found.keywords,
            ["anti-infantry 4+", "devastating wounds", "rapid fire 1"]
        );
    }

    #[test]
    fn a_melee_weapon_with_no_keywords_parses() {
        let found = weapon(&node(
            "w",
            "Gork\u{2019}s Klaw - strike",
            "Gork\u{2019}s Klaw - strike\nMelee Melee 6 2 14 -3 4",
        ));
        let stats = found.stats.unwrap();
        assert_eq!((stats.range.as_str(), stats.kind.as_str()), ("Melee", "Melee"));
        assert_eq!(stats.ap, "-3");
        assert!(found.keywords.is_empty());
    }

    #[test]
    fn a_passage_without_a_stat_line_keeps_its_text() {
        let found = weapon(&node("w", "Mystery", "Mystery\nsee datasheet"));
        assert!(found.stats.is_none());
        assert_eq!(found.text, "Mystery\nsee datasheet");
    }

    #[test]
    fn the_export_s_invulnerable_forms_become_a_save_or_none() {
        assert_eq!(invulnerable_save(Some("4")).as_deref(), Some("4+"));
        assert_eq!(invulnerable_save(Some("5")).as_deref(), Some("5+"));
        assert_eq!(invulnerable_save(Some("4*")).as_deref(), Some("4+"));
        assert_eq!(invulnerable_save(Some(" 6+ ")).as_deref(), Some("6+"));
        assert_eq!(invulnerable_save(Some("-")), None);
        assert_eq!(invulnerable_save(Some("")), None);
        assert_eq!(invulnerable_save(None), None);
    }

    #[test]
    fn the_invulnerable_note_loses_its_asterisk() {
        assert_eq!(
            invulnerable_note(Some("* Against ranged attacks only")).as_deref(),
            Some("Against ranged attacks only")
        );
        assert_eq!(
            invulnerable_note(Some(" * against ranged attacks only")).as_deref(),
            Some("against ranged attacks only")
        );
        assert_eq!(invulnerable_note(Some("")), None);
        assert_eq!(invulnerable_note(None), None);
    }

    #[test]
    fn points_split_description_from_cost() {
        let cost = points_cost(&node("c", "Cost 1", "5 models\n90"));
        assert_eq!(cost.description, "5 models");
        assert_eq!(cost.points, Some(90));
        let odd = points_cost(&node("c", "Cost 1", "1 model\nTBC"));
        assert_eq!(odd.points, None);
    }

    #[test]
    fn rules_text_drops_the_name_and_type_lines() {
        let ability = node(
            "a",
            "Might is Right",
            "Might is Right\nDatasheet\nWhile this model is leading a unit, add 1 to the Hit roll.",
        );
        assert_eq!(
            rules_text(&ability, Some("Datasheet")),
            "While this model is leading a unit, add 1 to the Hit roll."
        );
        let stratagem = node("s", "EXPLOSIVE CLEARANCE", "EXPLOSIVE CLEARANCE\nBattle Tactic\nWHEN: Your Shooting phase.");
        assert_eq!(
            rules_text(&stratagem, None),
            "Battle Tactic\nWHEN: Your Shooting phase."
        );
    }
}
