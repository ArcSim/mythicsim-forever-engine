//! Water Shield (408510), from Go sim/shaman/shields.go `registerWaterShieldSpell`: the cast puts
//! up three globes, each of which a hit the shaman takes (the shield self proc, which the gate
//! refuses) or a critical spell or heal of the shaman's own (MythicSim patch 145) spends for 2% of
//! maximum mana, one every 3.5 sec at most. The trigger waits a spell batch window, as Go's does,
//! and the globe is spent then if the cooldown still allows it. The last globe takes the shield
//! down.

use crate::core::fight::{Agent, AuraRef, Fight, SpellId, SpellResult};

#[derive(Clone, Debug)]
pub(crate) struct WaterShield {
    pub(crate) aura: AuraRef,
    globes: i32,
    mana_fraction: f64,
    icd: i64,
    timer: usize,
    /// The spells whose crits spend a globe, by spellbook position: the shaman's own spell
    /// damage and healing, Fire Nova aside, as preparation reads the trigger.
    triggers: Vec<bool>,
    metrics: usize,
}

pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    aura: &str,
    globes: i32,
    mana_fraction: f64,
    icd: i64,
    trigger_spells: &[usize],
    metrics: usize,
) -> Result<WaterShield, String> {
    let aura = fight.player_aura(aura)?;
    let mut triggers = vec![false; fight.spells.len()];
    for &spell in trigger_spells {
        *triggers
            .get_mut(spell)
            .ok_or_else(|| format!("Water Shield names spell {spell}"))? = true;
    }
    fight.timers.push(crate::core::time::STARTING_CD_TIME);
    let timer = fight.timers.len() - 1;
    Ok(WaterShield {
        aura,
        globes,
        mana_fraction,
        icd,
        timer,
        triggers,
        metrics,
    })
}

impl WaterShield {
    /// The cast: every globe up.
    pub(crate) fn cast<A: Agent>(&self, fight: &mut Fight<A>) {
        fight.activate_aura(self.aura);
        fight.set_stacks(self.aura, self.globes);
    }

    fn ready<A: Agent>(&self, fight: &Fight<A>) -> bool {
        fight.timers[self.timer] <= fight.now
    }

    /// The spell crit trigger: a critical direct spell hit or heal of the shaman's own, while
    /// the cooldown allows a globe, schedules the handler.
    pub(crate) fn on_hit<A: Agent>(
        &self,
        fight: &mut Fight<A>,
        spell: SpellId,
        result: &SpellResult,
    ) {
        if self.triggers[spell] && result.crit() && self.ready(fight) {
            fight.schedule_delayed_proc(self.aura, spell, *result);
        }
    }

    /// `spendGlobe`: a globe for its mana, if the cooldown and the shield still allow it.
    pub(crate) fn spend<A: Agent>(&self, fight: &mut Fight<A>) {
        if !self.ready(fight) || !fight.aura(self.aura).active {
            return;
        }
        fight.timers[self.timer] = fight.now + self.icd;
        fight.remove_stack(self.aura);
        let mana = fight.player.powers.max_mana * self.mana_fraction;
        fight.add_mana(mana, self.metrics);
    }
}
