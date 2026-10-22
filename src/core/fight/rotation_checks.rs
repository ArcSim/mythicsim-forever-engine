//! Go's validation messages of each rotation line (sim/core/apl.go `getStats`), and
//! MythicSim's Go CLI adapter that turns them into the rotation the application shows
//! (wowsim `mythicsim-forever/cli/report_rotation.go`).
//!
//! Go collects a line's messages while it builds the line's action and then its condition,
//! then while it finalizes and post-finalizes the actions it built. Rust's rotation compiler
//! follows Go's constructors, so building every line once more with the recorder on
//! ([`Fight::checks`]) gathers the messages of the lookups Go makes: a spell the character
//! does not know, an aura no unit has. The messages Go's constructors give for other reasons
//! are added where Rust builds the same action. Go gives the messages of values by their UUID
//! instead, which a request's rotation never sets, so no line has them.
//!
//! The adapter then drops the lines that cannot run (a spell the character lacks, a hidden
//! line, an Autocast Other Cooldowns with nothing to cast), takes the terms the engine drops
//! out of the remaining conditions, marks each missing aura's warning by what the engine did
//! with its check, and adds where a missing aura really is.

use serde_json::{json, Map, Value as Json};

use crate::contracts::prepared_v2::ActionId;
use crate::rotation::{
    compile_bool_value, compile_condition, Action as ParsedAction, CompiledCondition, Item,
    Rotation,
};

use super::log::action_string;
use super::{Agent, Fight, Side, SpellId};

/// Go `proto.LogLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Information,
    Warning,
    Error,
}

/// Go `proto.APLValidation`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) level: Level,
    pub(crate) text: String,
}

impl Check {
    fn warning(text: impl Into<String>) -> Check {
        Check {
            level: Level::Warning,
            text: text.into(),
        }
    }

    /// Go protojson: the zero level, Information, is left out.
    fn to_json(&self) -> Json {
        let mut object = Map::new();
        match self.level {
            Level::Information => {}
            Level::Warning => {
                object.insert("logLevel".into(), "Warning".into());
            }
            Level::Error => {
                object.insert("logLevel".into(), "Error".into());
            }
        }
        object.insert("validation".into(), self.text.clone().into());
        Json::Object(object)
    }
}

/// What the adapter adds to the player's metrics: the rotation that can run and the
/// validation messages of each of its lines, in the same order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Presentation {
    pub(crate) effective_rotation: Json,
    pub(crate) rotation_stats: Json,
}

impl Presentation {
    /// The adapter's `rotationPresentationVersion`.
    pub(crate) const VERSION: u32 = 1;

    /// Add the presentation to a player's metrics, as the adapter writes it.
    pub(crate) fn add_to(&self, player: &mut Map<String, Json>) {
        player.insert("effectiveRotation".into(), self.effective_rotation.clone());
        player.insert("rotationStats".into(), self.rotation_stats.clone());
        player.insert("rotationPresentationVersion".into(), Self::VERSION.into());
    }
}

/// One built line, for Go's finalizing: the action it was built of.
struct Built {
    parsed: ParsedAction,
}

impl<A: Agent> Fight<A> {
    /// Go `APLRotation.ValidationMessage`, while the recorder is on.
    pub(crate) fn check(&self, level: Level, text: impl FnOnce() -> String) {
        if self.checks.borrow().is_none() || crate::rotation::lookups_unreported() {
            return;
        }
        let text = text();
        if let Some(list) = self.checks.borrow_mut().as_mut() {
            list.push(Check { level, text });
        }
    }

    /// Go `GetAPLAura` and `GetAPLICDAura` on a unit without the aura.
    pub(crate) fn no_aura_found(&self, side: Side, id: &ActionId) {
        self.check(Level::Warning, || {
            format!(
                "No aura found on {} for: {}",
                self.label_of(side),
                action_string(id)
            )
        });
    }

    /// Go `GetAuraByID` on the player's pet at a position among its pets: whether it has the
    /// aura, false when there is no such pet, for which Go gives no message of the aura.
    pub(crate) fn pet_aura_known(&self, pet: usize, id: &ActionId) -> bool {
        let Some(auras) = self.pet_agent_auras.get(pet) else {
            return false;
        };
        let known = auras.contains(id);
        if !known {
            self.check(Level::Warning, || {
                format!(
                    "No aura found on {} for: {}",
                    self.pet_agent_labels[pet],
                    action_string(id)
                )
            });
        }
        known
    }

    /// Go `GetTargetAPLSpell` on the player: the spell of exactly the action.
    pub(crate) fn exact_spell(&self, id: &ActionId) -> Option<SpellId> {
        let spell = self
            .spells
            .iter()
            .position(|spell| spell.caster == Side::Player && &spell.id == id);
        if spell.is_none() {
            self.check(Level::Warning, || {
                format!(
                    "{} does not know spell {}",
                    self.config.player_label,
                    action_string(id)
                )
            });
        }
        spell
    }

    /// Run `f` with the recorder on: what it returns and the messages it gave.
    fn recording<T>(&self, f: impl FnOnce() -> T) -> (T, Vec<Check>) {
        let outer = self.checks.replace(Some(Vec::new()));
        let out = f();
        let checks = self.checks.replace(outer).unwrap_or_default();
        (out, checks)
    }

    /// The rotation as the adapter presents it: `config` is the request's rotation in
    /// protojson, `rotation` its parse.
    pub(crate) fn presentation(&self, rotation: &Rotation, config: &Json) -> Presentation {
        let mut stats = self.rotation_stats(rotation, config);
        self.add_aura_hints(&mut stats);
        let mut apl = config.clone();
        if let (Json::Object(apl), Json::Object(stats)) = (&mut apl, &mut stats) {
            clean_rotation(apl, stats);
        }
        Presentation {
            effective_rotation: apl,
            rotation_stats: stats,
        }
    }

    /// Go `APLRotation.getStats`: the validation messages of each prepull action, priority
    /// list item and group action, at its position in the request.
    fn rotation_stats(&self, rotation: &Rotation, config: &Json) -> Json {
        let list = |value: &Json, key: &str| -> Vec<Json> {
            value
                .get(key)
                .and_then(Json::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let hidden = |item: &Json| item.get("hide").and_then(Json::as_bool) == Some(true);
        let prepull_config = list(config, "prepullActions");
        let priority_config = list(config, "priorityList");
        let group_config = list(config, "groups");
        self.with_lookup(|lookup| {
            // Go builds every line, then finalizes the actions it built.
            let mut prepull: Vec<(Vec<Check>, Option<Built>)> = Vec::new();
            for (index, item) in prepull_config.iter().enumerate() {
                let position = index + 1;
                let parsed = rotation.prepull.iter().find(|p| p.position == position);
                let line = match parsed {
                    _ if hidden(item) => (Vec::new(), None),
                    // The parser skips a prepull action after the pull.
                    None => (
                        vec![Check::warning(
                            "Invalid time for 'Do At', ignoring this Prepull Action",
                        )],
                        None,
                    ),
                    Some(prepull) => {
                        let (built, checks) = self.recording(|| {
                            self.prepull_act(prepull, lookup, 0, &mut Vec::new(), &mut Vec::new())?;
                            // Go `newAPLAction`: the condition after the action; a constant
                            // false drops the action.
                            (compile_condition(prepull.condition.as_ref(), lookup)
                                != CompiledCondition::Pruned)
                                .then_some(())
                        });
                        (
                            checks,
                            built.map(|()| Built {
                                parsed: prepull.action.clone(),
                            }),
                        )
                    }
                };
                prepull.push(line);
            }
            let build = |item: Option<&Item>, in_group: bool| -> (Vec<Check>, Option<Built>) {
                let Some(item) = item else {
                    return (Vec::new(), None);
                };
                let (built, checks) = self.recording(|| {
                    self.compile_action(&item.action, lookup, None, &mut Vec::new())?;
                    if in_group {
                        // Go `newAPLActionWithGroupVars` keeps a constant condition.
                        compile_bool_value(item.condition.as_ref(), lookup);
                    } else if compile_condition(item.condition.as_ref(), lookup)
                        == CompiledCondition::Pruned
                    {
                        return None;
                    }
                    Some(())
                });
                (
                    checks,
                    built.map(|()| Built {
                        parsed: item.action.clone(),
                    }),
                )
            };
            let mut priority: Vec<(Vec<Check>, Option<Built>)> = (0..priority_config.len())
                .map(|index| {
                    let item = rotation
                        .priority_list
                        .iter()
                        .find(|item| item.position == index + 1);
                    build(item, false)
                })
                .collect();
            // Go builds each group's actions, then once more for each further reference to the
            // group in the priority list and for every reference to it from a group's action
            // (`newAPLRotation` appends the group's configuration again). Each build has its
            // own messages.
            let names: Vec<&str> = group_config
                .iter()
                .map(|group| group.get("name").and_then(Json::as_str).unwrap_or_default())
                .collect();
            let mut first_positions = Vec::new();
            let mut next_position = priority_config.len() + 1;
            for group in &group_config {
                first_positions.push(next_position);
                next_position += list(group, "actions").len();
            }
            let mut instances: Vec<usize> = (0..group_config.len()).collect();
            let mut claimed = vec![false; priority.len()];
            let mut groups: Vec<Vec<(Vec<Check>, Option<Built>)>> = Vec::new();
            // A group that reaches itself never finishes building in Go.
            while groups.len() < instances.len() && groups.len() < MAX_GROUP_BUILDS {
                let index = instances[groups.len()];
                let parsed = rotation.groups.get(index);
                let lines: Vec<(Vec<Check>, Option<Built>)> = (0..list(&group_config[index], "actions").len())
                    .map(|offset| {
                        let position = first_positions[index] + offset;
                        let item = parsed
                            .and_then(|g| g.items.iter().find(|item| item.position == position));
                        build(item, true)
                    })
                    .collect();
                let mut found = false;
                for (line, (_, built)) in priority.iter().enumerate() {
                    if referenced_group(built) == Some(names[index]) && !claimed[line] {
                        if found {
                            instances.push(index);
                        }
                        found = true;
                        claimed[line] = true;
                    }
                }
                for (_, built) in &lines {
                    if let Some(name) = referenced_group(built) {
                        if let Some(&other) = instances.iter().find(|&&other| names[other] == name) {
                            instances.push(other);
                        }
                    }
                }
                groups.push(lines);
            }
            // Go's finalizing and post-finalizing of each built action.
            let sequences: Vec<String> = priority
                .iter()
                .chain(groups.iter().flatten())
                .filter_map(|(_, built)| built.as_ref())
                .flat_map(|built| sequence_names(&built.parsed))
                .collect();
            for (checks, built) in prepull
                .iter_mut()
                .chain(priority.iter_mut())
                .chain(groups.iter_mut().flatten())
            {
                if let Some(built) = built {
                    self.finalize(built, &sequences, &names, checks);
                }
            }
            let to_json = |lines: &[(Vec<Check>, Option<Built>)]| -> Json {
                Json::Array(
                    lines
                        .iter()
                        .map(|(checks, _)| {
                            if checks.is_empty() {
                                json!({})
                            } else {
                                json!({"validations": checks.iter().map(Check::to_json).collect::<Vec<_>>()})
                            }
                        })
                        .collect(),
                )
            };
            // Go protojson leaves out an empty list.
            let mut stats = Map::new();
            if !prepull.is_empty() {
                stats.insert("prepullActions".into(), to_json(&prepull));
            }
            if !priority.is_empty() {
                stats.insert("priorityList".into(), to_json(&priority));
            }
            if !groups.is_empty() {
                stats.insert(
                    "groups".into(),
                    Json::Array(
                        groups
                            .iter()
                            .map(|lines| {
                                if lines.is_empty() {
                                    json!({})
                                } else {
                                    json!({"actions": to_json(lines)})
                                }
                            })
                            .collect(),
                    ),
                );
            }
            Json::Object(stats)
        })
    }

    /// Go `Finalize` and `PostFinalize` of a built action: a reset of a sequence no action
    /// runs, a reference to no group, and what the cooldown actions will cast.
    fn finalize(
        &self,
        built: &Built,
        sequences: &[String],
        groups: &[&str],
        checks: &mut Vec<Check>,
    ) {
        match &built.parsed {
            ParsedAction::ResetSequence(name) if !sequences.contains(name) => {
                checks.push(Check::warning(format!("No sequence with name: '{name}'")));
            }
            ParsedAction::GroupReference { name, .. } if !groups.contains(&name.as_str()) => {
                checks.push(Check {
                    level: Level::Error,
                    text: format!("Group reference '{name}' not found"),
                });
            }
            ParsedAction::AutocastOtherCooldowns => {
                checks.push(if self.major_cooldowns.is_empty() {
                    Check::warning("Autocast Other Cooldowns will not cast any spells! There are either no major cooldowns configured for this character, or all of them are manually cast in the APL.")
                } else {
                    Check {
                        level: Level::Information,
                        text: format!(
                            "Autocast Other Cooldowns will cast the following spells: {}",
                            self.action_list(
                                self.major_cooldowns
                                    .iter()
                                    .map(|cooldown| &self.spells[cooldown.spell].id)
                            )
                        ),
                    }
                });
            }
            ParsedAction::CastAllStatBuffCooldowns(stats) => {
                let action = format!("CastAllBuffCooldownsFor({})", stat_names(stats));
                let spells = self.cast_all_spells(stats);
                checks.push(if spells.is_empty() {
                    Check::warning(format!("{action} will not cast any spells! There are either no major cooldowns buffing the specified stat type(s), or all of them are manually cast in the APL."))
                } else {
                    Check {
                        level: Level::Information,
                        text: format!(
                            "{action} will cast the following spells: {}",
                            self.action_list(spells.iter().map(|&spell| &self.spells[spell].id))
                        ),
                    }
                });
            }
            // Go's schedule finalizes nothing of its inner action, and a sequence's steps are
            // casts, which finalize nothing.
            _ => {}
        }
    }

    /// Go `StringFromActionIDs`.
    fn action_list<'a>(&self, ids: impl Iterator<Item = &'a ActionId>) -> String {
        ids.map(action_string).collect::<Vec<_>>().join(", ")
    }

    /// The adapter's `addAuraHints`: after each "No aura found" warning, a note naming where
    /// the aura is, when the engine has it on another unit or under another tag.
    fn add_aura_hints(&self, stats: &mut Json) {
        let mut lists: Vec<&mut Json> = Vec::new();
        let Json::Object(stats) = stats else {
            return;
        };
        let mut groups = None;
        for (key, value) in stats.iter_mut() {
            match key.as_str() {
                "prepullActions" | "priorityList" => lists.push(value),
                "groups" => groups = Some(value),
                _ => {}
            }
        }
        if let Some(Json::Array(groups)) = groups {
            for group in groups {
                if let Some(actions) = group.get_mut("actions") {
                    lists.push(actions);
                }
            }
        }
        for list in lists {
            let Json::Array(notes) = list else { continue };
            for note in notes {
                let Some(Json::Array(warnings)) = note.get_mut("validations") else {
                    continue;
                };
                let hints: Vec<Json> = warnings
                    .iter()
                    .filter_map(|warning| {
                        let message = warning.get("validation")?.as_str()?;
                        let rest = message.strip_prefix("No aura found on ")?;
                        let at = rest.find(" for: ")?;
                        let (spell, tag) = aura_id(&rest[at + " for: ".len()..])?;
                        let hint = self.aura_hint(&rest[..at], spell, tag);
                        (!hint.is_empty()).then(|| json!({"validation": hint, "auraHint": true}))
                    })
                    .collect();
                warnings.extend(hints);
            }
        }
    }

    /// The adapter's `auraHint`: where an aura the rotation looked for on the unit of `label`
    /// really is, in the Rotation builder's words. Empty when no unit has it.
    fn aura_hint(&self, label: &str, spell: i32, tag: i32) -> String {
        let name = format!("{{SpellID: {spell}}}");
        let targets: Vec<Side> = (0..self.targets.len()).map(Side::target).collect();
        if label == self.config.player_label {
            for &target in &targets {
                if self.aura_with_spell(target, spell).is_some() {
                    let mut hint = format!(
                        "{name} is on the target, not on you: set \"Check aura or resource on\" to Current enemy target"
                    );
                    let id = ActionId {
                        spell_id: spell,
                        ..ActionId::default()
                    };
                    let has_dot = self.spells.iter().any(|s| {
                        s.caster == Side::Player
                            && s.id == id
                            && s.dot.is_some_and(|dot| self.dots[dot].side.is_target())
                    });
                    if has_dot {
                        hint += ", or use \"Your DoT remaining\" or \"Your DoT active\"";
                    }
                    return hint + ".";
                }
            }
            if let Some(found) = self.aura_with_spell(Side::Player, spell) {
                if found != tag {
                    return format!(
                        "{name} on you has tag {found}: set the aura's Tag to {found}."
                    );
                }
            }
            return String::new();
        }
        for &target in &targets {
            if self.label_of(target) != label {
                continue;
            }
            if let Some(found) = self.aura_with_spell(target, spell) {
                if found != tag {
                    return format!(
                        "{name} on the target has tag {found}: set the aura's Tag to {found}."
                    );
                }
            }
            if self.aura_with_spell(Side::Player, spell).is_some() {
                return format!(
                    "{name} is on you, not on the target: set \"Check aura or resource on\" to You."
                );
            }
        }
        // Pets and other units: no hint.
        String::new()
    }

    /// The adapter's `auraWithSpell`: the tag of the first aura of the spell registered on
    /// the unit, whatever its tag. Targets past the first carry the first's auras.
    fn aura_with_spell(&self, side: Side, spell: i32) -> Option<i32> {
        let source = if side.is_target() { Side::Target } else { side };
        self.trackers[source.index()].auras.iter().find_map(|aura| {
            aura.action_id
                .as_ref()
                .filter(|id| id.spell_id == spell)
                .map(|id| id.tag)
        })
    }
}

/// Go stops building groups that reference each other without end here.
const MAX_GROUP_BUILDS: usize = 256;

/// The group a built group reference names.
fn referenced_group(built: &Option<Built>) -> Option<&str> {
    match built {
        Some(Built {
            parsed: ParsedAction::GroupReference { name, .. },
        }) => Some(name.as_str()),
        _ => None,
    }
}

/// The sequence names a built action holds, which a reset of a sequence finds.
fn sequence_names(action: &ParsedAction) -> Vec<String> {
    match action {
        ParsedAction::Sequence { name, .. } => vec![name.clone()],
        ParsedAction::Schedule { inner, .. } => sequence_names(&inner.action),
        _ => Vec::new(),
    }
}

/// Go `StringFromStatTypes`.
pub(crate) fn stat_names(stats: &[i32]) -> String {
    stats
        .iter()
        .map(|&stat| {
            usize::try_from(stat)
                .ok()
                .filter(|&index| index < crate::prepare::stats::SIM_STATS_LEN)
                .map_or("", |index| {
                    crate::prepare::stats::Stat::from_index(index).name()
                })
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The adapter's `auraIDRe`: `{SpellID: N}` or `{SpellID: N, Tag: T}`.
fn aura_id(text: &str) -> Option<(i32, i32)> {
    let inner = text.strip_prefix("{SpellID: ")?.strip_suffix('}')?;
    let (spell, tag) = match inner.split_once(", Tag: ") {
        Some((spell, tag)) => (spell, tag.parse().ok()?),
        None => (inner, 0),
    };
    if spell.is_empty() || !spell.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((spell.parse().ok()?, tag))
}

/// Go `ActionID.String` of a protojson action ID, `None` where protojson reads no action ID.
fn id_string(value: &Json) -> Option<String> {
    crate::rotation::parse_action_id(value)
        .ok()
        .map(|id| action_string(&id))
}

/// The adapter's `missingID`.
fn missing_id(value: &Json, unavailable: &[String]) -> bool {
    id_string(value).is_some_and(|id| unavailable.contains(&id))
}

/// The adapter's `cleanRotation`: keep the lines that can run, with the notes of each.
fn clean_rotation(apl: &mut Map<String, Json>, stats: &mut Map<String, Json>) {
    for section in ["prepullActions", "priorityList"] {
        let entries = apl
            .get(section)
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_default();
        let notes = stats
            .get(section)
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_default();
        let mut kept = Vec::new();
        let mut kept_notes = Vec::new();
        for (index, entry) in entries.into_iter().enumerate() {
            let Json::Object(mut line) = entry else {
                continue;
            };
            if line.get("hide") == Some(&Json::Bool(true)) {
                continue;
            }
            let mut note = match notes.get(index) {
                Some(Json::Object(note)) => note.clone(),
                _ => Map::new(),
            };
            let warnings = note
                .get("validations")
                .and_then(Json::as_array)
                .cloned()
                .unwrap_or_default();
            let mut unavailable = Vec::new();
            let mut empty_cooldowns = false;
            for warning in &warnings {
                let message = warning
                    .get("validation")
                    .and_then(Json::as_str)
                    .unwrap_or_default();
                if let Some(at) = message.find("does not know spell ") {
                    unavailable.push(message[at + "does not know spell ".len()..].to_string());
                }
                if message.starts_with("No aura found on ") {
                    if let Some(at) = message.find(" for: ") {
                        unavailable.push(format!("aura:{}", &message[at + " for: ".len()..]));
                    }
                }
                if message.contains("Autocast Other Cooldowns will not cast any spells") {
                    empty_cooldowns = true;
                }
            }
            let Some(Json::Object(action)) = line.get_mut("action") else {
                continue;
            };
            if !clean_action(action, &unavailable, empty_cooldowns) {
                continue;
            }
            let action = Json::Object(action.clone());
            let mut remaining = Vec::new();
            for mut warning in warnings.iter().cloned() {
                let message = warning
                    .get("validation")
                    .and_then(Json::as_str)
                    .unwrap_or_default()
                    .to_string();
                if let Some(at) = message.find("does not know spell ") {
                    if !references_id(&action, &message[at + "does not know spell ".len()..]) {
                        continue;
                    }
                }
                if let (true, Some(at)) = (
                    message.starts_with("No aura found on "),
                    message.find(" for: "),
                ) {
                    let flag = if references_id(&action, &message[at + " for: ".len()..]) {
                        "missingAura"
                    } else {
                        "ignoredCheck"
                    };
                    if let Json::Object(warning) = &mut warning {
                        warning.insert(flag.into(), Json::Bool(true));
                    }
                }
                remaining.push(warning);
            }
            if !warnings.is_empty() {
                note.insert("validations".into(), Json::Array(remaining));
            }
            kept.push(Json::Object(line));
            kept_notes.push(Json::Object(note));
        }
        if apl.contains_key(section) {
            if kept.is_empty() {
                apl.remove(section);
                stats.remove(section);
            } else {
                apl.insert(section.into(), Json::Array(kept));
                stats.insert(section.into(), Json::Array(kept_notes));
            }
        }
    }
    // Groups have the same action-list shape, with validations in actions.
    let group_stats = stats
        .get_mut("groups")
        .and_then(Json::as_array_mut)
        .map(std::mem::take);
    let Some(mut group_stats) = group_stats else {
        return;
    };
    if let Some(Json::Array(groups)) = apl.get_mut("groups") {
        for (index, group) in groups.iter_mut().enumerate() {
            let Json::Object(group) = group else { continue };
            let Some(notes) = group_stats.get_mut(index) else {
                continue;
            };
            let mut a = Map::new();
            a.insert(
                "priorityList".into(),
                group.get("actions").cloned().unwrap_or(Json::Null),
            );
            let mut s = Map::new();
            s.insert(
                "priorityList".into(),
                notes.get("actions").cloned().unwrap_or(Json::Null),
            );
            clean_rotation(&mut a, &mut s);
            group.insert(
                "actions".into(),
                a.remove("priorityList").unwrap_or(Json::Null),
            );
            if let Json::Object(notes) = notes {
                notes.insert(
                    "actions".into(),
                    s.remove("priorityList").unwrap_or(Json::Null),
                );
            }
        }
    }
    stats.insert("groups".into(), Json::Array(group_stats));
}

/// The adapter's `cleanAction`: false when the line cannot run.
fn clean_action(
    action: &mut Map<String, Json>,
    unavailable: &[String],
    empty_cooldowns: bool,
) -> bool {
    for kind in [
        "castSpell",
        "castFriendlySpell",
        "channelSpell",
        "multidot",
        "multishield",
    ] {
        if let Some(Json::Object(cast)) = action.get(kind) {
            if cast
                .get("spellId")
                .is_some_and(|id| missing_id(id, unavailable))
            {
                return false;
            }
        }
    }
    if action.contains_key("autocastOtherCooldowns") && empty_cooldowns {
        return false;
    }
    if let Some(Json::Object(condition)) = action.get_mut("condition") {
        if !clean_value(condition, unavailable) {
            action.remove("condition");
        }
    }
    // Both sequence constructors discard unavailable subactions in the engine.
    for kind in ["sequence", "strictSequence"] {
        if let Some(Json::Object(sequence)) = action.get_mut(kind) {
            let actions = sequence
                .get("actions")
                .and_then(Json::as_array)
                .cloned()
                .unwrap_or_default();
            let mut kept = Vec::new();
            for step in actions {
                if let Json::Object(mut step) = step {
                    if clean_action(&mut step, unavailable, empty_cooldowns) {
                        kept.push(Json::Object(step));
                    }
                }
            }
            if kept.is_empty() {
                return false;
            }
            sequence.insert("actions".into(), Json::Array(kept));
        }
    }
    true
}

/// The adapter's `constantAuraValues`: the aura values the engine builds as a constant when
/// the aura is missing, whose checks stay in the line.
const CONSTANT_AURA_VALUES: [&str; 5] = [
    "auraIsActive",
    "auraIsInactive",
    "auraRemainingTime",
    "auraNumStacks",
    "auraIsKnown",
];

/// The adapter's `cleanValue`: false where the engine builds no value. Comparisons and `not`
/// of a missing spell disappear, `and` and `or` keep their remaining operands.
fn clean_value(value: &mut Map<String, Json>, unavailable: &[String]) -> bool {
    let kinds: Vec<String> = value.keys().cloned().collect();
    for kind in kinds {
        let Some(Json::Object(body)) = value.get_mut(&kind) else {
            continue;
        };
        if let Some(id) = body.get("auraId") {
            if let Some(id) = id_string(id) {
                if unavailable.contains(&format!("aura:{id}")) {
                    return CONSTANT_AURA_VALUES.contains(&kind.as_str());
                }
            }
        }
        if body
            .get("spellId")
            .is_some_and(|id| missing_id(id, unavailable))
        {
            return kind == "spellIsKnown";
        }
        if kind == "and" || kind == "or" {
            let values = body
                .get("vals")
                .and_then(Json::as_array)
                .cloned()
                .unwrap_or_default();
            let mut kept = Vec::new();
            for operand in values {
                if let Json::Object(mut operand) = operand {
                    if clean_value(&mut operand, unavailable) {
                        kept.push(Json::Object(operand));
                    }
                }
            }
            if kept.is_empty() {
                return false;
            }
            body.insert("vals".into(), Json::Array(kept));
        } else {
            for key in ["val", "lhs", "rhs"] {
                if let Some(Json::Object(child)) = body.get_mut(key) {
                    if !clean_value(child, unavailable) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// The adapter's `referencesID`: whether a spell or aura of the ID is anywhere in the value.
fn references_id(value: &Json, id: &str) -> bool {
    match value {
        Json::Object(map) => map.iter().any(|(key, child)| {
            ((key == "spellId" || key == "auraId") && id_string(child).as_deref() == Some(id))
                || references_id(child, id)
        }),
        Json::Array(items) => items.iter().any(|child| references_id(child, id)),
        _ => false,
    }
}
