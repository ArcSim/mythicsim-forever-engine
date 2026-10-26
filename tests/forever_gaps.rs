//! Production requests the Rust engine used to refuse, which then ran in the Go engine
//! (fixtures/forever-gaps): each prepares under Forever's rules, passes the gate and runs, and
//! the effect it lacked acts.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::Value;
use std::{fs, path::Path};

fn request(name: &str) -> Value {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/forever-gaps"
    ))
    .join(format!("{name}.request.json"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn prepared(name: &str, rules: Rules) -> Value {
    prepare_json_with(request(name).to_string().as_bytes(), "rust", rules).unwrap()
}

fn kinds(prepared: &Value) -> Vec<&str> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|effect| effect["kind"].as_str().unwrap())
        .collect()
}

/// The request under Forever's rules passes the gate; the player's metrics of one run.
fn run(name: &str) -> (Value, Value) {
    let value = prepared(name, Rules::Forever);
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = result["raidMetrics"]["parties"][0]["players"][0].clone();
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0, "{name}");
    (value, player)
}

/// Casts, damage and healing of the player's action with this spell and tag.
fn action(player: &Value, spell_id: i64, tag: i64) -> (f64, f64, f64) {
    let mut totals = (0.0, 0.0, 0.0);
    for action in player["actions"].as_array().unwrap() {
        let id = &action["id"];
        if id["spellId"].as_i64() != Some(spell_id) || id["tag"].as_i64().unwrap_or(0) != tag {
            continue;
        }
        for target in action["targets"].as_array().unwrap() {
            totals.0 += target["casts"].as_f64().unwrap_or(0.0);
            totals.1 += target["damage"].as_f64().unwrap_or(0.0);
            totals.2 += target["healing"].as_f64().unwrap_or(0.0);
        }
    }
    totals
}

/// A seal twisting tank: Seal of Light (20349) heals the paladin off its 15 PPM manager
/// (20340), and its judgement puts up Judgement of Light (20346). Go's exporter describes
/// neither.
#[test]
fn seal_of_light_heals_a_tank_and_judges_light() {
    let name = "protection-paladin-seal-of-light";
    assert!(!kinds(&prepared(name, Rules::GoReference)).contains(&"seal_of_light"));
    let (value, player) = run(name);
    let kinds = kinds(&value);
    assert!(kinds.contains(&"seal_of_light"), "{kinds:?}");
    assert!(kinds.contains(&"judgement_of_light"), "{kinds:?}");
    assert!(action(&player, 20349, 0).0 > 0.0, "Seal of Light casts");
    assert!(action(&player, 20340, 0).2 > 0.0, "Seal of Light heals");
    assert!(
        action(&player, 20346, 0).0 > 0.0,
        "Judgement of Light casts"
    );
}

/// A tank's own Retribution Aura rank 2 (10298): the cast turns the aura on, and the shield
/// (tag 2) strikes back at the target's landed swings.
#[test]
fn a_tanks_own_retribution_aura_strikes_back() {
    let (value, player) = run("protection-paladin-retribution-aura");
    assert!(kinds(&value).contains(&"paladin_aura_cast"));
    assert_eq!(action(&player, 10298, 0).0, 50.0, "one cast per iteration");
    assert!(action(&player, 10298, 2).1 > 0.0, "the shield's damage");
}

/// Thunder Clap beside a permanent slow in its exclusive category runs, its bid raised over
/// the slow it shares the category with.
#[test]
fn thunder_clap_beside_a_permanent_slow_runs() {
    let (_, player) = run("protection-warrior-thunder-clap");
    assert!(action(&player, 11581, 0).0 > 0.0, "Thunder Clap casts");
}

/// Manual Crowd Pummeler openers that move Windfury Weapon to the swap weapon, against the
/// party's Windfury Totem.
#[test]
fn a_windfury_weapon_swap_runs() {
    let (value, _) = run("enhancement-shaman-windfury-swap");
    assert!(kinds(&value).contains(&"windfury_weapon"));
}

/// Retribution swaps from the Manual Crowd Pummeler onto weapons with procs of their own.
#[test]
fn retribution_swaps_onto_weapons_with_procs_run() {
    run("retribution-paladin-swap-blade-of-hanna");
    let (_, player) = run("retribution-paladin-swap-sulfuras");
    assert!(action(&player, 21162, 0).1 > 0.0, "Sulfuras's Fireball");
}

/// The gain of the player's resource row with this spell and type.
fn resource_gain(player: &Value, spell_id: i64, kind: &str) -> f64 {
    player["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"]["spellId"].as_i64() == Some(spell_id) && row["type"] == kind)
        .map(|row| row["gain"].as_f64().unwrap_or(0.0))
        .sum()
}

/// Vampiric Embrace (15286): the cast on its cooldown puts the debuff up, and a batch window
/// after each Shadow hit or tick on the target the priest gains health. Go's exporter leaves
/// the cast unknown.
#[test]
fn vampiric_embrace_heals_the_priest() {
    let name = "shadow-priest-vampiric-embrace";
    assert!(!kinds(&prepared(name, Rules::GoReference)).contains(&"vampiric_embrace"));
    let (value, player) = run(name);
    assert!(kinds(&value).contains(&"vampiric_embrace"));
    assert!(action(&player, 15286, 0).0 > 50.0, "Vampiric Embrace casts");
    assert!(resource_gain(&player, 15286, "ResourceTypeHealth") > 0.0);
}

/// A strict sequence among the prepull actions takes control of the rotation, whose first
/// step waits on a time condition: each step casts once a fight.
#[test]
fn a_prepull_strict_sequence_runs_its_steps_once() {
    let (_, player) = run("shadow-priest-prepull-sequence");
    for spell in [20554, 14751, 1309636] {
        assert_eq!(action(&player, spell, 0).0, 50.0, "spell {spell}");
    }
}

/// Sequence steps with conditions of their own, as strict sequences have.
#[test]
fn sequence_steps_carry_conditions() {
    run("marksmanship-hunter-sequence-conditions");
}

/// Fourteen stat auras, more than the 4,096 combinations preparation used to read.
#[test]
fn fourteen_stat_auras_prepare() {
    let (value, _) = run("enhancement-shaman-many-stat-auras");
    assert!(kinds(&value).contains(&"stat_auras"));
}

/// A condition on a spell value that Go builds only for a known spell, which preparation used
/// to leave to the Go exporter.
#[test]
fn spell_values_fold_as_go_builds_them() {
    run("retribution-paladin-spell-value-fold");
}

/// A demon's mana percent and aura time read in the warlock's rotation, and an aura time
/// without an aura, which Go builds no value for.
#[test]
fn pet_and_missing_aura_values_run() {
    run("demonology-warlock-pet-values");
    run("demonology-warlock-pet-aura-time");
    run("combat-rogue-aura-value-without-aura");
}

/// Stormcaller's Garb (3): Lightning Bolt, Chain Lightning, Overload and Shock hits give
/// Stormcaller's Wrath (26121).
#[test]
fn stormcallers_garb_procs_its_wrath() {
    let (value, player) = run("enhancement-shaman-stormcallers-garb");
    assert!(value["effects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|effect| effect["kind"] == "spell_data_stat_proc"
            && effect["aura"] == "Stormcaller's Wrath"));
    let uptime = player["auras"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|aura| aura["id"]["spellId"].as_i64() == Some(26121))
        .map(|aura| aura["uptimeSecondsAvg"].as_f64().unwrap_or(0.0))
        .sum::<f64>();
    assert!(uptime > 0.0, "Stormcaller's Wrath uptime");
}

/// The Bear's Swipe (1264502) with three targets up cleaves them.
#[test]
fn a_bears_swipe_cleaves_three_targets() {
    let (_, player) = run("hunter-bear-swipe-three-targets");
    let pet = &player["pets"][0];
    assert!(action(pet, 1264502, 0).1 > 0.0, "Swipe damage");
}
