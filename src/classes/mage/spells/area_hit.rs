//! Arcane Explosion (10202), Cone of Cold (10161), Frost Nova (10230) and Blast Wave
//! (13021), from Go sim/mage/arcane_explosion.go, cone_of_cold.go, frost_nova.go and
//! blast_wave.go: `CalcAndDealAoeDamageWithVariance` with `OutcomeMagicHitAndCrit`, a
//! rolled hit calculated and dealt on each target in turn. Cone of Cold, Frost Nova and
//! Blast Wave are binary, which the spell's flags carry.

use crate::classes::mage::talents::winters_chill::WintersChill;
use crate::core::fight::{Agent, Fight, Side, SpellId};

/// `winters_chill` is the mage's talent: under client 1.60.1.70630 a Cone of Cold gains each
/// stack's share of spell damage (the same bonus damage its own coefficient reads, so Frost
/// damage and the target's school bonus count), on top of its coefficient.
pub(crate) fn apply<A: Agent>(
    fight: &mut Fight<A>,
    spell: SpellId,
    winters_chill: Option<&WintersChill>,
) {
    let share = match (winters_chill, fight.spells[spell].class_spell.as_deref()) {
        (Some(chill), Some("cone_of_cold")) => chill.cone_of_cold_coefficient(fight),
        _ => 0.0,
    };
    fight.calc_and_deal_aoe_damage_with_variance(
        spell,
        |fight| fight.roll_damage_effect(spell),
        move |fight: &mut Fight<A>, spell: SpellId, target: Side, base: f64| {
            let base = if share > 0.0 {
                share.mul_add(fight.bonus_damage(spell, target), base)
            } else {
                base
            };
            fight.calc_damage(spell, target, base)
        },
    );
}
