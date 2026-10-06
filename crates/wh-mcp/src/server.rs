//! The MCP server: one tool per question an agent asks, each a thin wrapper
//! over a `wh_ask::Bundle` call. The library does the work; this file chooses
//! names, defaults, and caps that suit a model's context window.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::Serialize;
use serde_json::{Map, Value};
use wh_ask::{AskError, Bundle, NodeRef, Subgraph, UnitRef};

use crate::params::{
    AbilityParams, DetachmentParams, DetachmentSection, FactionParams, FactionRulesParams,
    FactionSection, FindUnitsParams, KeywordParams, NeighborsParams, NodeParams, RosterParams,
    RuleKind, RuleUnitsParams, SearchParams, SubgraphParams, UnitParams, UnitSection,
};
use crate::reply;

const DEFAULT_SEARCH: usize = 10;
const MAX_SEARCH: usize = 50;
const DEFAULT_NEIGHBORS: usize = 50;
const MAX_NEIGHBORS: usize = 200;
const MAX_DEPTH: u32 = 2;
const MAX_SUBGRAPH_EDGES: usize = 100;
const MAX_SUBGRAPH_NODES: usize = 150;
const DEFAULT_LIST: usize = 200;
const MAX_LIST: usize = 1000;

/// A list that may have been cut. `summary` states the count in a sentence, so a
/// model reads the number instead of counting items, and `total` is the full
/// count, so an agent can tell a complete list from a partial one.
#[derive(Serialize)]
struct Listing<T> {
    summary: String,
    total: usize,
    truncated: bool,
    items: Vec<T>,
}

/// Cut `items` to the limit and describe them. `describe` gets the full count.
fn listing<T>(mut items: Vec<T>, limit: Option<usize>, describe: impl FnOnce(usize) -> String) -> Listing<T> {
    let limit = limit.unwrap_or(DEFAULT_LIST).clamp(1, MAX_LIST);
    let total = items.len();
    items.truncate(limit);
    let mut summary = describe(total);
    if total > limit {
        summary.push_str(&format!(" Showing the first {limit}; pass a larger limit for more."));
    }
    Listing {
        summary,
        total,
        truncated: total > limit,
        items,
    }
}

/// A roster with its size stated up front.
#[derive(Serialize)]
struct RosterReply {
    summary: String,
    count: usize,
    subject: NodeRef,
    units: Vec<UnitRef>,
}

/// `1 stratagem`, `2 stratagems`.
fn count_of(n: usize, noun: &str) -> String {
    let plural = match noun.strip_suffix('y') {
        Some(stem) => format!("{stem}ies"),
        None => format!("{noun}s"),
    };
    format!("{n} {}", if n == 1 { noun } else { plural.as_str() })
}

/// The server. It holds one open bundle, shared by every request.
#[derive(Clone)]
pub struct WhServer {
    bundle: Arc<Bundle>,
}

impl WhServer {
    /// A server over an open bundle.
    pub fn new(bundle: Arc<Bundle>) -> Self {
        Self { bundle }
    }
}

#[tool_router]
impl WhServer {
    #[tool(
        description = "Search the rules data by words in names and text, best match first. \
Use it when you do not know an exact name or want to find rules text. Each result has an id, \
kind, and name; pass the id to get_node, get_unit, or the other tools. kinds can limit the \
results, such as Datasheet, Ability, Stratagem, or Enhancement.",
        annotations(read_only_hint = true)
    )]
    async fn search(&self, Parameters(p): Parameters<SearchParams>) -> CallToolResult {
        let limit = p.limit.unwrap_or(DEFAULT_SEARCH).clamp(1, MAX_SEARCH);
        let kinds = strs(&p.kinds);
        reply::ok(&self.bundle.search(&p.query, &kinds, limit))
    }

    #[tool(
        description = "Turn a unit name into datasheet ids. Returns every datasheet with that \
exact name across factions, or the closest names when none match exactly. Use it before get_unit \
when a name might be shared or misspelled.",
        annotations(read_only_hint = true)
    )]
    async fn find_units(&self, Parameters(p): Parameters<FindUnitsParams>) -> CallToolResult {
        reply::ok(&self.bundle.find_units(&p.name))
    }

    #[tool(
        description = "List every faction with its id. Use the names with get_roster, \
list_detachments, and get_faction_rules.",
        annotations(read_only_hint = true)
    )]
    async fn list_factions(&self) -> CallToolResult {
        reply::ok(&self.bundle.factions())
    }

    #[tool(
        description = "List a faction's detachments. Boarding Actions variants can share a name \
with a standard detachment in the same faction; the id tells them apart.",
        annotations(read_only_hint = true)
    )]
    async fn list_detachments(&self, Parameters(p): Parameters<FactionParams>) -> CallToolResult {
        reply::from(self.bundle.detachments(&p.faction))
    }

    #[tool(
        description = "List every unit a faction, a Space Marine or Chaos Space Marine chapter, or \
a Legiones Daemonica god (Khorne, Tzeentch, Nurgle, Slaanesh) can field. A chapter or a god also \
includes the generic parent units it can take. Takes a name or id; list_factions gives faction names.",
        annotations(read_only_hint = true)
    )]
    async fn get_roster(&self, Parameters(p): Parameters<RosterParams>) -> CallToolResult {
        reply::from(self.bundle.roster(&p.subject).map(|roster| {
            let count = roster.units.len();
            RosterReply {
                summary: format!("{} can field {}.", roster.subject.name, count_of(count, "unit")),
                count,
                subject: roster.subject,
                units: roster.units,
            }
        }))
    }

    #[tool(
        description = "Get one unit's datasheet: its own text (loadout, transport capacity, leader \
rules, damaged profile), keywords, model stats including the invulnerable save, composition, \
points, abilities, wargear with weapon stats, wargear options, the units it can lead, and the \
leaders that can join it. Takes a name or id. A name several units share is an error listing \
their ids; report that more than one exists and cover each. Pass sections to get only some. enhancements and detachment_rules are only returned \
when asked for.",
        annotations(read_only_hint = true)
    )]
    async fn get_unit(&self, Parameters(p): Parameters<UnitParams>) -> CallToolResult {
        reply::from(self.unit_json(p))
    }

    #[tool(
        description = "Get a detachment's stratagems (with command point cost, turn, and phase), its own rules, and its enhancements (with points cost). \
The reply states how many of each there are. Pass sections to get only some. A name several detachments share is an error listing their ids; \
report that more than one exists and cover each.",
        annotations(read_only_hint = true)
    )]
    async fn get_detachment(&self, Parameters(p): Parameters<DetachmentParams>) -> CallToolResult {
        reply::from(self.detachment_json(p))
    }

    #[tool(
        description = "Get a faction's army-wide abilities. Pass sections to also get every \
stratagem or enhancement across its detachments, which are long lists.",
        annotations(read_only_hint = true)
    )]
    async fn get_faction_rules(
        &self,
        Parameters(p): Parameters<FactionRulesParams>,
    ) -> CallToolResult {
        reply::from(self.faction_json(p))
    }

    #[tool(
        description = "List the units with a keyword, such as Infantry, Psyker, or Fly. Common \
keywords have hundreds of units, so the reply gives the total and returns up to limit of them \
(default 200); truncated says when the list was cut.",
        annotations(read_only_hint = true)
    )]
    async fn units_with_keyword(&self, Parameters(p): Parameters<KeywordParams>) -> CallToolResult {
        reply::from(self.bundle.units_with_keyword(&p.keyword).map(|units| {
            listing(units, p.limit, |total| {
                let verb = if total == 1 { "has" } else { "have" };
                format!("{} {verb} the keyword {}.", count_of(total, "unit"), p.keyword)
            })
        }))
    }

    #[tool(
        description = "List the units with an ability and each unit's own value for it, such as \
Feel No Pain 5+ or Deadly Demise D6. The reply gives the total and returns up to limit rows \
(default 200); truncated says when the list was cut.",
        annotations(read_only_hint = true)
    )]
    async fn units_with_ability(&self, Parameters(p): Parameters<AbilityParams>) -> CallToolResult {
        reply::from(self.bundle.units_with_ability(&p.ability).map(|holders| {
            let units = holders
                .iter()
                .map(|holder| holder.unit.id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len();
            listing(holders, p.limit, |entries| {
                let verb = if units == 1 { "has" } else { "have" };
                let mut summary = format!("{} {verb} the ability {}.", count_of(units, "unit"), p.ability);
                if entries != units {
                    summary.push_str(&format!(" It appears {entries} times, because some units have several values."));
                }
                summary
            })
        }))
    }

    #[tool(
        description = "List the units an enhancement can be given to (kind enhancement), or the \
units a detachment rule names (kind detachment_rule), such as who gets Idols of Khorne. \
Enhancement names repeat across detachments; a shared name is an error listing each id and its \
detachment, and you should report that more than one exists.",
        annotations(read_only_hint = true)
    )]
    async fn units_for_rule(&self, Parameters(p): Parameters<RuleUnitsParams>) -> CallToolResult {
        reply::from(match p.kind {
            RuleKind::Enhancement => self.bundle.enhancement_units(&p.name),
            RuleKind::DetachmentRule => self.bundle.detachment_rule_units(&p.name),
        })
    }

    #[tool(
        description = "Get one node of the rules graph by id: its kind, text, and attributes. Use \
it for an id another tool returned.",
        annotations(read_only_hint = true)
    )]
    async fn get_node(&self, Parameters(p): Parameters<NodeParams>) -> CallToolResult {
        reply::from(self.bundle.node_detail(&p.id))
    }

    #[tool(
        description = "List the nodes one edge away from a node, with each edge's kind and \
direction (outgoing means the node is the edge's source). Pass edge_kinds to limit them; \
bundle_info lists the kinds. Default 50, at most 200.",
        annotations(read_only_hint = true)
    )]
    async fn get_neighbors(&self, Parameters(p): Parameters<NeighborsParams>) -> CallToolResult {
        let limit = p.limit.unwrap_or(DEFAULT_NEIGHBORS).clamp(1, MAX_NEIGHBORS);
        let kinds = strs(&p.edge_kinds);
        reply::from(self.bundle.neighbors(&p.id, &kinds, limit))
    }

    #[tool(
        description = "Advanced: the nodes and edges within 1 or 2 edges of one or more seed ids. \
Try get_neighbors first. The result is capped, and truncated says when it was cut.",
        annotations(read_only_hint = true)
    )]
    async fn get_subgraph(&self, Parameters(p): Parameters<SubgraphParams>) -> CallToolResult {
        reply::from(self.subgraph(p))
    }

    #[tool(
        description = "Describe the loaded data: the edition, and how many nodes and edges of \
each kind there are. Use it to learn the valid kinds for search, get_neighbors, and get_subgraph.",
        annotations(read_only_hint = true)
    )]
    async fn bundle_info(&self) -> CallToolResult {
        reply::ok(self.bundle.manifest())
    }
}

/// What an agent is told once, when it connects. It is written out in the
/// `#[tool_handler]` attribute below, because that macro takes a string literal.
#[tool_handler(
    name = "wh-mcp",
    version = "0.1.0",
    instructions = "Read-only access to Wahapedia's Warhammer 40,000 10th-edition datasheets, rules, and army structure. \
Start with get_unit for a question about one unit, get_roster for a faction, chapter, or daemon-god \
question, and get_detachment for stratagems, rules, and enhancements. \
Names must be exact, though case, 'the', and apostrophe style do not matter. \
When a name matches nothing, the error lists close names. \
When a name matches several things, the error lists their ids and what tells them apart. \
Do not pick one silently: tell the user that more than one exists, and either ask which is meant or answer for each by calling again with each id. \
find_units and search help you find the right name. \
Results that list things state how many there are, in a summary or a count field. Read that number; do not count the items yourself. \
Every result is JSON. An empty list means the source data has none, not that the call failed: \
some units, such as Angron, have no leader entries in the source export. \
When you show results to a person, credit Wahapedia, and use the wahapedia_link fields where there are any."
)]
impl ServerHandler for WhServer {}

/// The work behind the tools that take `sections`. Each returns the whole reply
/// or the library's error, so a name problem reaches the agent unchanged.
impl WhServer {
    fn unit_json(&self, p: UnitParams) -> Result<Value, AskError> {
        let card = self.bundle.unit(&p.unit)?;
        let mut out = Map::new();
        put(&mut out, "unit", &card.unit);
        for section in sections_or(p.sections, UnitSection::CARD) {
            match section {
                UnitSection::Text => put(&mut out, "text", &card.text),
                UnitSection::Keywords => put(&mut out, "keywords", &card.keywords),
                UnitSection::Models => put(&mut out, "models", &card.models),
                UnitSection::Composition => put(&mut out, "composition", &card.composition),
                UnitSection::Points => put(&mut out, "points", &card.points),
                UnitSection::Abilities => put(&mut out, "abilities", &card.abilities),
                UnitSection::Wargear => put(&mut out, "wargear", &card.wargear),
                UnitSection::Options => put(&mut out, "options", &card.options),
                UnitSection::Leads => put(&mut out, "leads", &card.leads),
                UnitSection::LedBy => put(&mut out, "led_by", &card.led_by),
                UnitSection::Enhancements => put(
                    &mut out,
                    "enhancements",
                    &self.bundle.unit_enhancements(&card.unit.id)?,
                ),
                UnitSection::DetachmentRules => put(
                    &mut out,
                    "detachment_rules",
                    &self.bundle.unit_detachment_rules(&card.unit.id)?,
                ),
            }
        }
        Ok(Value::Object(out))
    }

    fn detachment_json(&self, p: DetachmentParams) -> Result<Value, AskError> {
        let mut sections = Map::new();
        let mut counts = Vec::new();
        for section in sections_or(p.sections, DetachmentSection::ALL) {
            match section {
                DetachmentSection::Stratagems => {
                    let found = self.bundle.detachment_stratagems(&p.detachment)?;
                    counts.push(count_of(found.len(), "stratagem"));
                    put(&mut sections, "stratagems", &found);
                }
                DetachmentSection::Rules => {
                    let found = self.bundle.detachment_rules(&p.detachment)?;
                    counts.push(count_of(found.len(), "rule"));
                    put(&mut sections, "rules", &found);
                }
                DetachmentSection::Enhancements => {
                    let found = self.bundle.detachment_enhancements(&p.detachment)?;
                    counts.push(count_of(found.len(), "enhancement"));
                    put(&mut sections, "enhancements", &found);
                }
            }
        }
        Ok(with_summary("This detachment has", &counts, sections))
    }

    fn faction_json(&self, p: FactionRulesParams) -> Result<Value, AskError> {
        let mut sections = Map::new();
        let mut counts = Vec::new();
        for section in sections_or(p.sections, FactionSection::DEFAULT) {
            match section {
                FactionSection::Abilities => {
                    let found = self.bundle.faction_abilities(&p.faction)?;
                    counts.push(count_of(found.len(), "ability"));
                    put(&mut sections, "abilities", &found);
                }
                FactionSection::Stratagems => {
                    let found = self.bundle.faction_stratagems(&p.faction)?;
                    counts.push(count_of(found.len(), "stratagem"));
                    put(&mut sections, "stratagems", &found);
                }
                FactionSection::Enhancements => {
                    let found = self.bundle.faction_enhancements(&p.faction)?;
                    counts.push(count_of(found.len(), "enhancement"));
                    put(&mut sections, "enhancements", &found);
                }
            }
        }
        Ok(with_summary("This faction has", &counts, sections))
    }

    /// The library allows 2000 edges, which is too much for a model's context, so
    /// this cuts the reply to a few hundred and says so.
    fn subgraph(&self, p: SubgraphParams) -> Result<Subgraph, AskError> {
        let seeds = p.seeds.iter().map(String::as_str).collect::<Vec<_>>();
        let kinds = strs(&p.edge_kinds);
        let depth = p.depth.unwrap_or(1).clamp(1, MAX_DEPTH);
        let mut graph = self.bundle.subgraph(&seeds, &kinds, depth)?;
        if graph.edges.len() > MAX_SUBGRAPH_EDGES {
            graph.edges.truncate(MAX_SUBGRAPH_EDGES);
            graph.truncated = true;
        }
        if graph.truncated {
            graph.nodes.truncate(MAX_SUBGRAPH_NODES);
        }
        Ok(graph)
    }
}

/// `sections` with a `summary` sentence and a `counts` list ahead of them, so the
/// numbers are the first thing a model reads.
fn with_summary(lead: &str, counts: &[String], sections: Map<String, Value>) -> Value {
    let mut out = Map::new();
    out.insert("summary".to_string(), Value::String(format!("{lead} {}.", join_counts(counts))));
    out.insert("counts".to_string(), Value::Array(counts.iter().cloned().map(Value::String).collect()));
    out.extend(sections);
    Value::Object(out)
}

/// `a`, `a and b`, `a, b and c`.
fn join_counts(counts: &[String]) -> String {
    match counts {
        [] => "nothing".to_string(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A list of strings as the `&str` slice the library takes.
fn strs(list: &Option<Vec<String>>) -> Vec<&str> {
    list.iter().flatten().map(String::as_str).collect()
}

/// The sections asked for, or the default when none (or an empty list) were.
fn sections_or<S: Copy>(given: Option<Vec<S>>, default: &[S]) -> Vec<S> {
    match given {
        Some(list) if !list.is_empty() => list,
        _ => default.to_vec(),
    }
}

/// Add `value` to the reply under `key`. Our results are plain data, so encoding
/// them cannot fail; a null would only mark a bug.
fn put<T: Serialize>(out: &mut Map<String, Value>, key: &str, value: &T) {
    out.insert(
        key.to_string(),
        serde_json::to_value(value).unwrap_or(Value::Null),
    );
}
