//! Incinerate (1293813), from Go sim/warlock/incinerate.go: the client damage roll, the magic hit
//! and crit, the whole hit raised on a target burning with Immolate (Go
//! `IncinerateImmolateBonusOnWholeHit`, MythicSim patch 155), dealt when it arrives.

use crate::core::fight::{Agent, DotId, Fight, Side, SpellId};

pub(crate) fn apply<A: Agent>(
    fight: &mut Fight<A>,
    spell: SpellId,
    target: Side,
    immolate: Option<DotId>,
    immolate_bonus: f64,
) {
    let base = fight.roll_damage_effect(spell);
    let immolated = immolate.is_some_and(|dot| {
        fight
            .aura(fight.dots[fight.dot_on(dot, target)].aura)
            .active
    });
    let mut result = fight.calc_damage(spell, target, base);
    if immolated {
        fight.scale_damage(spell, &mut result, immolate_bonus);
    }
    fight.deal_damage_after_travel(spell, result);
}
