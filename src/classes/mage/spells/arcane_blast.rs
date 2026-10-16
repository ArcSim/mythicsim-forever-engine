//! Arcane Blast (1239700) and its Arcane Blast buff (400573), from Go sim/mage/arcane_blast.go
//! and arcane_charge.go, as client 70425 redesigned them. Each cast stacks the buff up to four
//! times, from its first stack for 15 sec: a stack does not refresh it and nothing spends it.
//! Each stack gives the mage 15% of its Intellect as spell damage, which the `stat_auras`
//! combinations hold, and raises the cost of Arcane Blast itself.

use crate::core::fight::{Agent, AuraRef, Fight, ModId, ModKind, Side, SpellId};

#[derive(Clone, Debug)]
pub(crate) struct ArcaneCharges {
    pub(crate) aura: AuraRef,
    cost_per_stack: f64,
    cost_mod: ModId,
    /// The stacks' place in the stat aura combinations: offset and width.
    stat_place: (u32, u32),
}

pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    aura: &str,
    cost_per_stack: f64,
    stat_place: (u32, u32),
) -> Result<ArcaneCharges, String> {
    let aura = fight.player_aura(aura)?;
    let blasts = fight.spells_with_class(&["arcane_blast"]);
    let cost_mod = fight.register_mod(ModKind::PowerCostPercentAdd, 0.0, 0, blasts);
    Ok(ArcaneCharges {
        aura,
        cost_per_stack,
        cost_mod,
        stat_place,
    })
}

/// Go Arcane Blast `ApplyEffects`: the hit lands at once, then the buff starts if it is down and
/// gains a stack.
pub(crate) fn apply<A: Agent>(
    fight: &mut Fight<A>,
    spell: SpellId,
    target: Side,
    charges: &ArcaneCharges,
) {
    let base = fight.roll_damage_effect(spell);
    let result = fight.calc_damage(spell, target, base);
    fight.deal_damage(spell, result, false);
    if !fight.aura(charges.aura).active {
        fight.activate_aura(charges.aura);
    }
    fight.add_stack(charges.aura);
}

impl ArcaneCharges {
    fn set_stat_stacks<A: Agent>(&self, fight: &mut Fight<A>, stacks: i32) {
        let (offset, width) = self.stat_place;
        let field = ((1u32 << width) - 1) << offset;
        fight.set_stat_mask((fight.stat_mask & !field) | ((stacks as u32) << offset));
    }

    pub(crate) fn on_gain<A: Agent>(&self, fight: &mut Fight<A>) {
        fight.activate_mod(self.cost_mod);
    }

    pub(crate) fn on_expire<A: Agent>(&self, fight: &mut Fight<A>) {
        self.set_stat_stacks(fight, 0);
        fight.deactivate_mod(self.cost_mod);
    }

    pub(crate) fn on_stacks_change<A: Agent>(&self, fight: &mut Fight<A>, stacks: i32) {
        self.set_stat_stacks(fight, stacks);
        fight.update_mod_value(self.cost_mod, self.cost_per_stack * f64::from(stacks));
    }
}
