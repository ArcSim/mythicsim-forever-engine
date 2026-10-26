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
