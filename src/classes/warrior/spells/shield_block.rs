//! Shield Block (2565), from Go sim/warrior/shield_block.go: in Defensive Stance with a shield,
//! the cast activates its aura at the maximum stacks. The aura's block chance rides on the stat
//! aura combinations, and its trigger, which acts at once, spends a stack on each hit the
//! warrior blocks.

use crate::{
    contracts::prepared_v2::Effect,
    core::fight::{Agent, AuraRef, Fight, SpellResult, OUTCOME_BLOCK},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ShieldBlock {
    pub(crate) aura: AuraRef,
    /// The `ExtraCastCondition`'s shield.
    pub(crate) can_block: bool,
    stat_bit: u32,
}

impl ShieldBlock {
    pub(crate) fn bind<A: Agent>(
        fight: &Fight<A>,
        effects: &[Effect],
        aura: &str,
        can_block: bool,
    ) -> Result<Self, String> {
        let stat_bit = Fight::<A>::stat_aura_bit(effects, aura)
            .ok_or_else(|| format!("{aura} is not a stat aura"))?;
        Ok(ShieldBlock {
            aura: fight.player_aura(aura)?,
            can_block,
            stat_bit,
        })
    }

    /// The cast's `ApplyEffects`: the aura, then its maximum stacks.
    pub(crate) fn apply<A: Agent>(self, fight: &mut Fight<A>) {
        fight.activate_aura(self.aura);
        let max = fight.aura(self.aura).max_stacks;
        fight.set_stacks(self.aura, max);
    }

    /// "Shield Block - Consume": a blocked hit taken spends a stack.
    pub(crate) fn on_hit_taken<A: Agent>(self, fight: &mut Fight<A>, result: &SpellResult) {
        if result.outcome & OUTCOME_BLOCK != 0 {
            fight.remove_stack(self.aura);
        }
    }

    /// `AttachStatBuff`'s gain and expiry.
    pub(crate) fn toggle<A: Agent>(self, fight: &mut Fight<A>, active: bool) {
        fight.set_stat_aura(self.stat_bit, active);
    }
}
