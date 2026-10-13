//! Go sim/common/forever/frost_oil.go: Frost Oil as a weapon imbue (the fork's patch 126).

use super::aura_helpers::{CallbackMask, HitOutcome, ProcTrigger};
use super::env::Environment;
use super::shared_items::spell_data_proc_damage_spell_config;
use super::spell::{ProcMask, SpellFlag};
use super::spelldata::must_find;

/// Frost Oil's use spell and item: the imbue IDs a request may name it by.
const USE_SPELL: i32 = 3595;
const ITEM: i32 = 3829;
/// The Frostbolt temporary enchantment 26 casts, at the 10% chance the client states for it.
const BOLT: i32 = 350025;

/// Go `core.StaticImbueProcs`: whether the imbue is a temporary enchantment whose effect is a
/// combat proc.
pub(crate) fn is_static_imbue_proc(imbue_id: i32) -> bool {
    imbue_id == USE_SPELL || imbue_id == ITEM
}

/// Go `registerFrostOil`: a flat chance on each landed hit of the oiled weapon to cast the
/// Frostbolt at the target. `hand` is `ProcMask::MELEE_MH` or `ProcMask::MELEE_OH`.
pub(crate) fn register_frost_oil(env: &mut Environment, hand: ProcMask) {
    let unit = env.player;
    let row = must_find(BOLT);
    let config = spell_data_proc_damage_spell_config(env, row);
    let bolt = env.sim.get_or_register_spell(unit, config);
    env.sim.spell_mut(bolt).flags |=
        SpellFlag::NO_ON_CAST_COMPLETE | SpellFlag::PASSIVE_SPELL | SpellFlag::PROC;
    let chance = f64::from(row.proc_chance) / 100.0;

    let name = if hand == ProcMask::MELEE_OH {
        "Frost Oil (off hand)"
    } else {
        "Frost Oil (main hand)"
    };
    env.sim.make_proc_trigger_aura(
        unit,
        &ProcTrigger {
            name: name.to_string(),
            callback: CallbackMask::ON_SPELL_HIT_DEALT,
            proc_mask: hand,
            is_weapon_proc: true,
            outcome: HitOutcome::LANDED,
            proc_chance: chance,
            trigger_immediately: true,
            ..ProcTrigger::default()
        },
    );
}
