//! Hellfire (11684), from Go sim/warlock/hellfire.go. The channel is an area dot on the
//! warlock. Each tick rolls a fixed amount, to hit and to crit, on every target before
//! dealing any, deals the batch, then burns the warlock for the same amount. The channel does
//! not stop at low health: the simulation has no healer and Life Tap already spends past
//! zero, and stopping clipped every channel after about 18 seconds into one-tick recasts at full
//! mana cost (#735, `afe6a8f09b`).

use crate::{
    classes::warlock::agent::WarlockAgent,
    core::fight::{DotId, Fight, Outcome, SpellId},
};

/// Hellfire's tick amount and whether its area hits crit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hellfire {
    tick_base: f64,
    tick_can_crit: bool,
}

/// Bind the tick. A client row that forbids the burn's crit picks Go's `NoHitCounter`
/// outcome, which Rust does not simulate, so the gate refuses a rotation that casts Hellfire.
pub(crate) fn bind(tick_base: f64, tick_can_crit: bool) -> Hellfire {
    Hellfire {
        tick_base,
        tick_can_crit,
    }
}

/// The channel cast: `AOEDot().Apply`.
pub(crate) fn apply_channel(fight: &mut Fight<WarlockAgent>, spell: SpellId) {
    let dot = fight.spells[spell].dot.expect("Hellfire has a channel dot");
    fight.apply_dot(dot);
}

/// The area dot's `OnTick`.
pub(crate) fn tick(fight: &mut Fight<WarlockAgent>, dot: DotId) {
    let hellfire = fight.agent.hellfire.expect("Hellfire is bound");
    assert!(
        hellfire.tick_can_crit,
        "the gate refuses a Hellfire whose burn cannot crit"
    );
    let spell = fight.dots[dot].spell;
    let results =
        fight.calc_periodic_aoe_damage(dot, hellfire.tick_base, Outcome::TickMagicHitAndCrit);
    fight.deal_batched_periodic_damage(spell, &results);
    fight.remove_health(hellfire.tick_base);
}
