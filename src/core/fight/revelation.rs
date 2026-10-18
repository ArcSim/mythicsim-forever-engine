//! Go sim/common/forever/revelation.go Enchant Weapon - Revelation (MythicSim patches 157 and
//! 170): a trigger on hits and heals and one on ticks. On an eligible landed hit that does not
//! crit and deals something, each rolls its chance, the patch 170 fit times one less the
//! player's spell crit, under its own label, and a spell batch window later activates the
//! class's buff. The buff adds its crit to the spells its mask names; the next cast of one that is
//! no channel or proc spends it, and the end of a channel it names ends it.

use crate::contracts::prepared_v2::Effect;

use super::spell_mod::{ModId, ModKind};
use super::{Agent, AuraRef, DotId, Fight, SpellId, SpellResult, OUTCOME_CRIT, OUTCOME_LANDED};

/// The bound enchant: its auras, the spells each part hears and the buff's crit modifier.
#[derive(Clone, Debug)]
pub(crate) struct Revelation {
    hit_trigger: AuraRef,
    buff: AuraRef,
    /// By spell: whether its hits and heals, or its ticks, can trigger it.
    hits: Vec<bool>,
    ticks: Vec<bool>,
    /// By spell: whether a cast of it spends the buff, and whether its dots' end ends it.
    consuming: Vec<bool>,
    channels: Vec<bool>,
    crit_mod: ModId,
    chance_at_no_crit: f64,
    delay: i64,
}

impl<A: Agent> Fight<A> {
    /// Bind Revelation to its auras and spells, and register the buff's crit modifier.
    pub(crate) fn bind_revelation(&mut self, effect: &Effect) -> Result<(), String> {
        let Effect::Revelation {
            trigger_aura,
            aura,
            hit_spells,
            tick_spells,
            affected_spells,
            consuming_spells,
            channel_spells,
            bonus_crit_percent,
            chance_at_no_crit,
            delay_ns,
            ..
        } = effect
        else {
            return Ok(());
        };
        let count = self.spells.len();
        let by_spell = |positions: &[usize]| -> Result<Vec<bool>, String> {
            let mut out = vec![false; count];
            for &position in positions {
                *out.get_mut(position)
                    .ok_or_else(|| format!("Revelation names spell {position}"))? = true;
            }
            Ok(out)
        };
        let (hits, ticks) = (by_spell(hit_spells)?, by_spell(tick_spells)?);
        let (consuming, channels) = (by_spell(consuming_spells)?, by_spell(channel_spells)?);
        by_spell(affected_spells)?;
        let crit_mod = self.register_mod(
            ModKind::BonusCritPercent,
            *bonus_crit_percent,
            0,
            affected_spells.clone(),
        );
        self.revelation = Some(Revelation {
            hit_trigger: self.player_aura(trigger_aura)?,
            buff: self.player_aura(aura)?,
            hits,
            ticks,
            consuming,
            channels,
            crit_mod,
            chance_at_no_crit: *chance_at_no_crit,
            delay: *delay_ns,
        });
        Ok(())
    }

    /// Go `AttachProcTriggerCallback` with revelation.go's `ExtraCondition`: a landed hit, heal
    /// or tick of a spell the trigger hears that does not crit and deals something rolls the
    /// chance at the player's spell crit, and the handler runs a batch window later.
    pub(crate) fn revelation_callback(
        &mut self,
        aura: AuraRef,
        spell: SpellId,
        result: &SpellResult,
    ) {
        let revelation = self.revelation.as_ref().expect("Revelation is bound");
        let heard = if aura == revelation.hit_trigger {
            &revelation.hits
        } else {
            &revelation.ticks
        };
        if !heard.get(spell).copied().unwrap_or(false)
            || result.outcome & OUTCOME_LANDED == 0
            || result.outcome & OUTCOME_CRIT != 0
            || result.damage <= 0.0
        {
            return;
        }
        // Go `RevelationChance`: the fit times one less the crit, clamped to 0..1.
        let crit = (self.player.powers.spell_crit_percent / 100.0).clamp(0.0, 1.0);
        let chance = revelation.chance_at_no_crit * (1.0 - crit);
        let delay = revelation.delay;
        if chance >= 1.0 || self.random_for_aura(aura) <= chance {
            self.schedule_delayed_proc_at(self.now + delay, aura, spell, *result);
        }
    }

    /// The trigger's handler: the buff, refreshed when it holds.
    pub(crate) fn revelation_handler(&mut self) {
        let buff = self.revelation.as_ref().expect("Revelation is bound").buff;
        self.activate_aura(buff);
    }

    /// The buff's crit on its spells as it is gained and fades.
    pub(crate) fn revelation_buff_changed(&mut self, gained: bool) {
        let crit_mod = self
            .revelation
            .as_ref()
            .expect("Revelation is bound")
            .crit_mod;
        if gained {
            self.activate_mod(crit_mod);
        } else {
            self.deactivate_mod(crit_mod);
        }
    }

    /// The buff's `OnCastComplete`: a cast of a spell it names that is no channel or proc spends
    /// it, after the cast's hit.
    pub(crate) fn revelation_cast_complete(&mut self, spell: SpellId) {
        let revelation = self.revelation.as_ref().expect("Revelation is bound");
        if revelation.consuming.get(spell).copied().unwrap_or(false) {
            let buff = revelation.buff;
            self.deactivate_aura(buff);
        }
    }

    /// The `OnExpire` revelation.go adds to every dot of a channel the buff names: the channel's
    /// end ends the buff.
    pub(crate) fn revelation_dot_expired(&mut self, dot: DotId) {
        let Some(revelation) = self.revelation.as_ref() else {
            return;
        };
        if revelation
            .channels
            .get(self.dots[dot].spell)
            .copied()
            .unwrap_or(false)
        {
            let buff = revelation.buff;
            self.deactivate_aura(buff);
        }
    }
}
