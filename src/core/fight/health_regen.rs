//! An aura that restores the wearer's health on a period while it holds: Runeblade of Baron
//! Rivendare's Unholy Aura (Go classic items_weapons.go), whose gain starts a periodic action
//! and whose expiry cancels it.

use crate::contracts::prepared_v2::Effect;

use super::{Action, Agent, AuraRef, Fight, ResourceKind, PRIORITY_AUTO};

/// One regenerating aura: its period, the health a tick gives and its metrics, and the
/// generation of the periodic action its last gain started.
#[derive(Clone, Debug)]
pub(crate) struct HealthRegen {
    aura: AuraRef,
    period: i64,
    health: f64,
    metrics: usize,
    generation: u32,
}

impl<A: Agent> Fight<A> {
    /// Bind the regenerating auras.
    pub(crate) fn bind_health_regens(&mut self, effects: &[Effect]) -> Result<(), String> {
        for effect in effects {
            if let Effect::HealthRegenAura {
                aura,
                period_ns,
                health,
                metrics_action_id,
            } = effect
            {
                let aura = self.player_aura(aura)?;
                let metrics =
                    self.new_resource_metrics(metrics_action_id.clone(), ResourceKind::Health);
                self.health_regens.push(HealthRegen {
                    aura,
                    period: *period_ns,
                    health: *health,
                    metrics,
                    generation: 0,
                });
            }
        }
        Ok(())
    }

    /// Go `StartPeriodicAction` on the aura's gain: the first tick a period later.
    pub(crate) fn health_regen_gained(&mut self, aura: AuraRef) {
        for index in 0..self.health_regens.len() {
            if self.health_regens[index].aura != aura {
                continue;
            }
            let regen = &mut self.health_regens[index];
            regen.generation = regen.generation.wrapping_add(1);
            let (at, generation) = (self.now + regen.period, regen.generation);
            self.schedule(
                at,
                PRIORITY_AUTO,
                Action::HealthRegenTick {
                    regen: index,
                    generation,
                },
            );
        }
    }

    /// The aura's expiry cancels the periodic action.
    pub(crate) fn health_regen_expired(&mut self, aura: AuraRef) {
        for regen in &mut self.health_regens {
            if regen.aura == aura {
                regen.generation = regen.generation.wrapping_add(1);
            }
        }
    }

    /// A tick of a periodic action the aura's last gain started: the health, and the next tick.
    pub(crate) fn health_regen_tick(&mut self, index: usize, generation: u32) {
        let regen = &self.health_regens[index];
        if regen.generation != generation || !self.aura(regen.aura).active {
            return;
        }
        let (health, metrics, period) = (regen.health, regen.metrics, regen.period);
        self.gain_health(health, metrics);
        self.schedule(
            self.now + period,
            PRIORITY_AUTO,
            Action::HealthRegenTick {
                regen: index,
                generation,
            },
        );
    }
}
