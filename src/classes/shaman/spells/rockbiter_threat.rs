//! Rockbiter Weapon Proc (20871), from Go sim/shaman/weapon_imbues.go `registerRockbiterThreat`
//! (patch 181, ported from wowsims/forever f128c3d268): Rockbiter Weapon (Passive) casts it off
//! every landed white swing and melee ability of a weapon carrying Rockbiter. Its script effect
//! adds 72 threat for every second of the main hand's swing, through the normal threat path, so the
//! shaman's threat multiplier applies.

use crate::core::fight::{Agent, Fight, Outcome, Side, SpellId, SpellResult};

/// The hands whose weapons carry Rockbiter and the flat threat of a cast, as the weapons worn say.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RockbiterThreat {
    pub(crate) spell: SpellId,
    pub(crate) main_hand: bool,
    pub(crate) off_hand: bool,
    pub(crate) threat_per_swing_second: f64,
    pub(crate) main_hand_swing_speed: f64,
}

impl RockbiterThreat {
    /// Go `shaman.MainHand().SwingSpeed * rockbiterThreatPerSwingSec`, the flat threat of a cast.
    fn flat_threat(&self) -> f64 {
        self.threat_per_swing_second * self.main_hand_swing_speed
    }

    /// The aura's OnSpellHitDealt: a landed hit of a spell whose proc mask holds a hand that
    /// carries Rockbiter casts the threat spell on the hit's target. `melee_main_hand` and
    /// `melee_off_hand` say whether the struck spell's proc mask holds the hand's melee bits.
    pub(crate) fn on_spell_hit_dealt<A: Agent>(
        &self,
        fight: &mut Fight<A>,
        melee_main_hand: bool,
        melee_off_hand: bool,
        result: &SpellResult,
    ) {
        if !result.landed() {
            return;
        }
        if (self.main_hand && melee_main_hand) || (self.off_hand && melee_off_hand) {
            fight.cast(self.spell, result.target);
        }
    }

    /// The spell's `ApplyEffects`: Go `CalcAndDealOutcome` with `OutcomeAlwaysHit`, after the
    /// flat threat is set from the main hand's speed.
    pub(crate) fn apply<A: Agent>(&self, fight: &mut Fight<A>, spell: SpellId, target: Side) {
        fight.spells[spell].flat_threat_bonus = self.flat_threat();
        let result = fight.calc_outcome(spell, target, Outcome::AlwaysHit);
        fight.deal_damage(spell, result, false);
    }
}
