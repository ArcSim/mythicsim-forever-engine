//! Rust preparation of a Warrior against the pinned Go exporter: one request for each build,
//! with the buffs, consumables and gear effects stripped away, and the sections of the exporter's
//! prepared state it wrote for that request.

use serde_json::Value;
use std::{fs, path::Path};

const SECTIONS: [&str; 8] = [
    "player.spells",
    "player.auras",
    "target.auras",
    "player.stats",
    "player.pseudo_stats",
    "melee",
    "effects",
    "player.major_cooldowns",
];

fn data(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/classes/warrior/prepare")
        .join(name);
    fs::read(path).unwrap()
}

fn section<'a>(document: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(document, |value, key| &value[key])
}

/// The paths at which two values differ.
fn differences(go: &Value, rust: &Value, path: &str, out: &mut Vec<String>) {
    match (go, rust) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                match b.get(key) {
                    Some(other) => differences(value, other, &format!("{path}/{key}"), out),
                    None => out.push(format!("{path}/{key}: missing in Rust")),
                }
            }
            for key in b.keys().filter(|key| !a.contains_key(*key)) {
                out.push(format!("{path}/{key}: only in Rust"));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!(
                    "{path}: {} items in Go, {} in Rust",
                    a.len(),
                    b.len()
                ));
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                differences(x, y, &format!("{path}/{i}"), out);
            }
        }
        (Value::Number(a), Value::Number(b)) if a.as_f64() == b.as_f64() => {}
        (a, b) if a == b => {}
        (a, b) => out.push(format!("{path}: Go {a}, Rust {b}")),
    }
}

fn check(name: &str) {
    let rust = forever_engine::prepare_json(&data(&format!("{name}.request.json")), name)
        .unwrap_or_else(|err| panic!("{name} is not prepared: {err}"));
    let golden: Value = serde_json::from_slice(&data(&format!("{name}.golden.json"))).unwrap();
    let mut diffs = Vec::new();
    for path in SECTIONS {
        differences(&golden[path], section(&rust, path), path, &mut diffs);
    }
    assert!(
        diffs.is_empty(),
        "{name} differs from Go:\n{}",
        diffs.join("\n")
    );
}

#[test]
fn arms_matches_go() {
    check("arms");
}

#[test]
fn fury_matches_go() {
    check("fury");
}

#[test]
fn protection_matches_go() {
    check("protection");
}

/// A Protection warrior tanking the target is prepared with the target's swing at it; the
/// fixture harness compares the tanking fixtures with Go.
#[test]
fn a_tanking_warrior_is_prepared_with_the_target_swing() {
    let mut request: Value = serde_json::from_slice(&data("protection.request.json")).unwrap();
    request["raid"]["tanks"] = serde_json::json!([{"index": 0, "type": "Player"}]);
    let prepared = forever_engine::prepare_json(&serde_json::to_vec(&request).unwrap(), "tank")
        .expect("a tanking warrior is prepared");
    assert!(prepared["enemy"].is_object());
}

/// Stance snapshots change when a stance's effects start, which the export cannot describe.
#[test]
fn stance_snapshots_are_refused() {
    let mut request: Value = serde_json::from_slice(&data("arms.request.json")).unwrap();
    request["raid"]["parties"][0]["players"][0]["dpsWarrior"]["options"]["classOptions"]
        ["stanceSnapshot"] = Value::Bool(true);
    let error = forever_engine::prepare_json(&serde_json::to_vec(&request).unwrap(), "snapshot")
        .expect_err("stance snapshots are refused");
    match error {
        forever_engine::PrepareError::Refused(refusal) => assert_eq!(refusal.code, "unrepresented"),
        other => panic!("{other}"),
    }
}

/// The arms request prepared after `edit` changes it.
fn prepared_after(edit: impl Fn(&mut Value)) -> Value {
    let mut request: Value = serde_json::from_slice(&data("arms.request.json")).unwrap();
    edit(&mut request);
    let bytes = serde_json::to_vec(&request).unwrap();
    forever_engine::prepare_json(&bytes, "arms").unwrap()
}

fn aura<'a>(prepared: &'a Value, label: &str) -> Option<&'a Value> {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["label"] == label)
}

/// Go patch 125: the boss suppresses the first 1% of melee and ranged hit unless the encounter
/// drops it. The values are the exporter's for the same requests.
#[test]
fn the_encounter_drops_the_boss_hit_suppression_when_it_asks() {
    assert_eq!(prepared_after(|_| {})["melee"]["hit_suppression"], 0.01);
    let dropped = prepared_after(|request| {
        request["encounter"]["noHitSuppression"] = true.into();
    });
    assert_eq!(dropped["melee"]["hit_suppression"], 0.0);
}

/// Go patch 127: each Battle Squawk stack is 5% melee attack speed, multiplied as Go's `math.Pow`
/// rounds it, and a party never has more than five.
#[test]
fn battle_squawk_stacks_multiply_the_melee_speed() {
    let base = prepared_after(|_| {})["melee"]["melee_speed_multiplier"]
        .as_f64()
        .unwrap();
    assert!(aura(&prepared_after(|_| {}), "Battle Squawk").is_none());
    for (squawks, stacks, factor) in [
        (1, 1, 1.05),
        (3, 3, 1.1576250000000001),
        (5, 5, 1.2762815625000001),
        (7, 5, 1.2762815625000001),
    ] {
        let prepared = prepared_after(|request| {
            request["raid"]["parties"][0]["battleSquawks"] = squawks.into();
        });
        let squawk = aura(&prepared, "Battle Squawk").expect("the aura");
        assert_eq!(squawk["stacks"], stacks, "{squawks} squawks");
        assert_eq!(squawk["active"], true);
        assert_eq!(squawk["duration_ns"], 240_000_000_000_u64);
        assert_eq!(
            prepared["melee"]["melee_speed_multiplier"]
                .as_f64()
                .unwrap(),
            base * factor,
            "{squawks} squawks"
        );
    }
}

/// Go patch 126: an oil in either imbue slot registers the Frostbolt of enchantment 26 and a
/// proc aura for the hand it is on.
#[test]
fn frost_oil_registers_a_proc_for_its_hand() {
    let prepared = prepared_after(|request| {
        let consumables = &mut request["raid"]["parties"][0]["players"][0]["consumables"];
        consumables["mhImbueId"] = 3595.into();
        consumables["ohTempEnchantId"] = 3829.into();
    });
    assert!(aura(&prepared, "Frost Oil (main hand)").is_some());
    assert!(aura(&prepared, "Frost Oil (off hand)").is_some());
    let spells = prepared["player"]["spells"].as_array().unwrap();
    assert_eq!(
        spells
            .iter()
            .filter(|spell| spell["action_id"]["spell_id"] == 350025)
            .count(),
        1
    );
}
