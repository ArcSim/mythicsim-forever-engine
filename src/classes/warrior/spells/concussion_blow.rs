//! Concussion Blow (12809), from Go sim/warrior/talents_protection.go: a special hit outcome with
//! no damage on the target, refunded on a miss. Its stun has no effect in scope, so the spell
//! changes no target's swing and its threat is the spell's flat bonus of none. The rage cost,
//! cooldown and range are the common cast's.

use crate::core::fight::{melee::PhysicalOutcome, Agent, Fight, Outcome, Side, SpellId};

/// The spell's `ApplyEffects`: Go `CalcAndDealOutcome` with `OutcomeMeleeSpecialHit`, and the
/// refund when it does not land.
pub(crate) fn apply<A: Agent>(fight: &mut Fight<A>, spell: SpellId, target: Side) {
    let outcome = Outcome::Table(PhysicalOutcome::MeleeSpecialHit { count: true });
    let result = fight.calc_outcome(spell, target, outcome);
    fight.deal_damage(spell, result, false);
    if !result.landed() {
        fight.issue_refund(spell);
    }
}
