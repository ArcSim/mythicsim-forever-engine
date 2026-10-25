//! A healer's gear under Forever's rules (`prepare::healer_gear`, README Rust-only mechanics):
//! the set bonuses, relics and trinkets the Go reference counts for nothing, read from their
//! client rows, and the set listeners it registers that Rust used to refuse. Each test puts the
//! gear on one of `fixtures/healers`' requests and checks what the export carries, then that the
//! request passes the gate and heals. None of it reaches the Go reference's rules.

use forever_engine::{check_prepared, prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// A healer fixture's request with `items` put in their equipment slots, by slot index.
fn request(name: &str, items: &[(usize, i64)]) -> Value {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/healers"))
        .join(format!("{name}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let equipment = &mut request["raid"]["parties"][0]["players"][0]["equipment"]["items"];
    let slots = equipment.as_array_mut().unwrap();
    for &(slot, id) in items {
        while slots.len() <= slot {
            slots.push(json!({}));
        }
        slots[slot] = json!({ "id": id });
    }
    request["simOptions"]["iterations"] = json!(20);
    request
}

fn prepared(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules).unwrap()
}

fn effects<'a>(prepared: &'a Value, kind: &str) -> Vec<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|effect| effect["kind"] == kind)
        .collect()
}

/// The positions of the player's spells with these client spell IDs.
fn positions(prepared: &Value, ids: &[i64]) -> Vec<usize> {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, spell)| ids.contains(&spell["action_id"]["spell_id"].as_i64().unwrap_or(0)))
        .map(|(i, _)| i)
        .collect()
}

/// The prepared request passes the gate and heals.
fn heals(prepared: &Value) -> Value {
    let prepared = serde_json::from_value(prepared.clone()).unwrap();
    check_prepared(&prepared).unwrap();
    let result = simulate_prepared(&prepared).unwrap().result;
    let hps = result["raidMetrics"]["parties"][0]["players"][0]["hps"]["avg"]
        .as_f64()
        .unwrap();
    assert!(hps > 100.0, "healing per second {hps}");
    result
}

/// The direct heal's average of a rank in the heals effect.
fn direct_average(prepared: &Value, spell_id: i64) -> f64 {
    effects(prepared, "heals")[0]["heals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|heal| heal["spell_id"] == spell_id)
        .unwrap()["direct"]["average"]
        .as_f64()
        .unwrap()
}

fn heal(prepared: &Value, spell_id: i64) -> &Value {
    effects(prepared, "heals")[0]["heals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|heal| heal["spell_id"] == spell_id)
        .unwrap()
}

/// Vestments of the Devout (5): Sudden Insight, a cast's 5% chance to restore 200 mana. Rust
/// refused a healer wearing it; now each heal's cast is heard.
#[test]
fn sudden_insight_restores_mana_on_a_healers_casts() {
    let devout = [(0, 16693), (2, 16695), (4, 16690), (6, 16692), (8, 16694)];
    let request = request("holy-priest", &devout);
    let forever = prepared(&request, Rules::Forever);
    let listeners = effects(&forever, "mana_on_cast");
    assert_eq!(listeners.len(), 1, "{listeners:?}");
    let listener = listeners[0];
    assert_eq!(listener["trigger_aura"], "Vestments of the Devout 5P");
    assert_eq!(listener["proc_chance"], 0.05);
    let greater_heal = positions(&forever, &[25314])[0];
    assert!(listener["spells"]
        .as_array()
        .unwrap()
        .iter()
        .any(|spell| spell["spell"] == greater_heal && spell["mana"] == 200.0));
    let result = heals(&forever);
    let gained = result["raidMetrics"]["parties"][0]["players"][0]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["id"]["spellId"] == 450576)
        .expect("Sudden Insight's mana metrics");
    assert!(gained["events"].as_f64().unwrap() > 0.0, "{gained}");
    assert!(effects(&prepared(&request, Rules::GoReference), "mana_on_cast").is_empty());
}

/// Totem of Life (27855): Lesser Healing Wave heals for up to 80 more.
#[test]
fn totem_of_life_adds_to_lesser_healing_wave() {
    let plain = prepared(&request("restoration-shaman", &[]), Rules::Forever);
    let totem = prepared(
        &request("restoration-shaman", &[(16, 22396)]),
        Rules::Forever,
    );
    for rank in [8004, 10468] {
        assert_eq!(
            direct_average(&totem, rank),
            direct_average(&plain, rank) + 80.0
        );
    }
    assert_eq!(direct_average(&totem, 25357), direct_average(&plain, 25357));
    heals(&totem);
}

/// Hazza'rah's Charm of Healing: on use, Rapid Healing (24546) for 15 sec, Greater Heal 40%
/// faster and the heals its row names 5% cheaper.
#[test]
fn hazzarahs_charm_speeds_greater_heal_on_use() {
    let forever = prepared(&request("holy-priest", &[(12, 19958)]), Rules::Forever);
    let used = effects(&forever, "spell_mod_aura_on_use");
    assert_eq!(used.len(), 1, "{used:?}");
    assert_eq!(used[0]["aura"], "Rapid Healing");
    let changes = used[0]["changes"].as_array().unwrap();
    for greater_heal in positions(&forever, &[2060, 10963, 10964, 10965, 25314]) {
        assert!(changes
            .contains(&json!({"spell": greater_heal, "kind": "cast_time_percent", "value": -0.4})));
        assert!(changes.contains(
            &json!({"spell": greater_heal, "kind": "power_cost_percent_add", "value": -0.05})
        ));
    }
    let result = heals(&forever);
    let rapid = result["raidMetrics"]["parties"][0]["players"][0]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["id"]["spellId"] == 24546)
        .expect("Rapid Healing's metrics");
    let uptime = rapid["uptimeSecondsAvg"].as_f64().unwrap();
    assert!(
        (15.0..=30.0).contains(&uptime),
        "Rapid Healing up {uptime} sec a fight"
    );
}

/// Vestments of Prophecy, a set the reference does not register: its client rows give Flash
/// Heal 0.1 sec off its cast and global cooldown (3), 2% spell crit (5) and Prayer of Healing
/// 25% crit (8).
#[test]
fn vestments_of_prophecy_reads_its_client_rows() {
    let prophecy = [
        (0, 16813),
        (2, 16816),
        (4, 16815),
        (5, 16819),
        (6, 16812),
        (7, 16817),
        (8, 16814),
        (9, 16811),
    ];
    let plain = prepared(&request("holy-priest", &[]), Rules::Forever);
    let set = prepared(&request("holy-priest", &prophecy), Rules::Forever);
    let spell = |prepared: &Value, id: i64| -> Value {
        prepared["player"]["spells"][positions(prepared, &[id])[0]].clone()
    };
    let flash = |prepared: &Value| spell(prepared, 10917)["default_cast"].clone();
    assert_eq!(
        flash(&set)["cast_time_ns"].as_i64().unwrap(),
        flash(&plain)["cast_time_ns"].as_i64().unwrap() - 100_000_000
    );
    assert_eq!(
        flash(&set)["gcd_ns"].as_i64().unwrap(),
        flash(&plain)["gcd_ns"].as_i64().unwrap() - 100_000_000
    );
    let crit = |prepared: &Value| {
        spell(prepared, 25316)["bonus_crit_percent"]
            .as_f64()
            .unwrap()
    };
    assert_eq!(crit(&set), crit(&plain) + 25.0);
    let spell_crit = |prepared: &Value| {
        prepared["player"]["stats"]["SpellCritPercent"]
            .as_f64()
            .unwrap_or(0.0)
    };
    assert!(spell_crit(&set) > spell_crit(&plain));
    heals(&set);
    let reference = prepared(&request("holy-priest", &prophecy), Rules::GoReference);
    assert_eq!(crit(&reference), crit(&plain));
}

/// The Earthfury (8): Healing Wave jumps to two more players at 20% of the heal before each;
/// (5) Mana Surge's 25% chance on Healing Wave and Lesser Healing Wave to return 35% of the
/// spell's base cost.
#[test]
fn the_earthfury_chains_healing_wave_and_returns_mana() {
    let earthfury = [
        (0, 16842),
        (2, 16844),
        (4, 16841),
        (5, 16840),
        (6, 16839),
        (7, 16838),
        (8, 16843),
        (9, 16837),
    ];
    let forever = prepared(&request("restoration-shaman", &earthfury), Rules::Forever);
    let wave = heal(&forever, 25357);
    assert_eq!(wave["targets"], 3);
    assert_eq!(wave["jump_multiplier"], 0.2);
    assert_eq!(wave["smart"], "chain");
    let surge = effects(&forever, "mana_on_cast");
    assert_eq!(surge.len(), 1, "{surge:?}");
    assert_eq!(surge[0]["proc_chance"], 0.25);
    let lesser = positions(&forever, &[10468])[0];
    let cost = forever["player"]["spells"][lesser]["cost"].clone();
    let entry = surge[0]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|spell| spell["spell"] == lesser)
        .expect("Lesser Healing Wave returns mana");
    assert!(entry["mana"].as_f64().unwrap() > 0.0, "{entry} {cost}");
    heals(&forever);
    let reference = prepared(
        &request("restoration-shaman", &earthfury),
        Rules::GoReference,
    );
    assert_eq!(heal(&reference, 25357)["targets"], 1);
}

/// The Elements (5): The Furious Storm, a cast's 4% chance at 65 spell power for 10 sec. Rust
/// refused a healer wearing it.
#[test]
fn the_furious_storm_procs_on_a_healers_casts() {
    let elements = [(0, 16667), (2, 16669), (4, 16666), (6, 16672), (8, 16668)];
    let forever = prepared(&request("restoration-shaman", &elements), Rules::Forever);
    let procs: Vec<&Value> = effects(&forever, "spell_data_stat_proc")
        .into_iter()
        .filter(|effect| effect["trigger_aura"] == "The Elements 5P")
        .collect();
    assert_eq!(procs.len(), 1);
    assert_eq!(procs[0]["aura"], "The Furious Storm");
    assert_eq!(procs[0]["proc_chance"], 0.04);
    assert!(!procs[0]["trigger_spells"].as_array().unwrap().is_empty());
    heals(&forever);
}
