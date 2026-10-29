//! Go `sim/warlock/weapon_stones.go`: Forever's Firestone and Spellstone are weapon imbues
//! rather than held off-hand items, and they stack with a temporary enchantment such as Wizard
//! Oil (Blizzard's Rogue and Warlock deep dive, 8 October 2026). The imbue is the Create spell's
//! triggered aura at its highest rank, read from client data: Firestone (23483) adds Fire damage
//! and spell critical strike chance, Spellstone (1237165) adds damage to the schools in its mask
//! (Fire and Shadow at the pinned build, mask 36; Shadow alone under Forever's rules, below) and
//! cast speed.

use crate::contracts::prepared_v2::ActionId;
use crate::prepare::dbcenums;
use crate::prepare::sim::{AuraConfig, BuildPhase, Rules, Sim, UnitId, NEVER_EXPIRES};
use crate::prepare::spelldata::Spell as Row;
use crate::prepare::stats::{Stat, Stats};

use super::spell_data::spell_data;
use super::Warlock;

/// The school damage stat each bit of an `A_MOD_DAMAGE_DONE` school mask raises.
const SCHOOL_DAMAGE_STATS: [(i32, Stat); 6] = [
    (2, Stat::HolyDamage),
    (4, Stat::FireDamage),
    (8, Stat::NatureDamage),
    (16, Stat::FrostDamage),
    (32, Stat::ShadowDamage),
    (64, Stat::ArcaneDamage),
];

/// The Create Spellstone triggered auras (`create_spellstone_triggered`).
const SPELLSTONE_RANKS: [i32; 3] = [1237162, 1237164, 1237165];

/// Shadow, the school mask client 1.60.1.70630 gives the Spellstone's damage effect.
const SPELLSTONE_SCHOOLS_70630: i32 = 32;

/// Go `weaponStoneStats`: the imbue's school damage and spell crit as stats; its cast speed goes
/// through the spell's speed pseudo stats.
///
/// Client 1.60.1.70630 narrows the Spellstone ranks' school mask (SpellEffect 1233765, 1233768 and
/// 1233770 of 1237162, 1237164 and 1237165, "Increase Shadow Dam") from 36 to 32: Shadow only, no
/// longer Fire. The frozen data (70425) still says 36, which the Go reference keeps.
fn weapon_stone_stats(imbue: &Row, rules: Rules) -> Stats {
    let mut bonus = Stats::default();
    for e in &imbue.effects {
        if e.effect_type != dbcenums::E_APPLY_AURA {
            continue;
        }
        if e.aura == dbcenums::A_MOD_DAMAGE_DONE {
            let schools = if rules == Rules::Forever && SPELLSTONE_RANKS.contains(&imbue.id) {
                SPELLSTONE_SCHOOLS_70630
            } else {
                e.misc
            };
            for (mask, stat) in SCHOOL_DAMAGE_STATS {
                if schools & mask != 0 {
                    bonus[stat] += e.base_value();
                }
            }
        } else if e.aura == dbcenums::A_MOD_SPELL_CRIT_CHANCE {
            bonus[Stat::SpellCritPercent] += e.base_value();
        }
    }
    bonus
}

impl Warlock {
    /// Go `registerWeaponStone`.
    pub(super) fn register_weapon_stone(&self, sim: &mut Sim, unit: UnitId) {
        let imbue = match self.options.enum_name("weapon_stone").as_str() {
            "Firestone" => spell_data().create_firestone_triggered.highest(),
            "Spellstone" => spell_data().create_spellstone_triggered.highest(),
            _ => return,
        };
        let bonus = weapon_stone_stats(imbue, sim.rules);
        let aura = sim.register_aura(
            unit,
            AuraConfig {
                label: imbue.name.clone(),
                action_id: Some(ActionId {
                    spell_id: imbue.id,
                    ..ActionId::default()
                }),
                build_phase: BuildPhase::GEAR,
                duration: NEVER_EXPIRES,
                ..AuraConfig::default()
            },
        );
        let aura = sim.make_permanent(aura);
        sim.attach_stats_buff(aura, bonus);
        sim.attach_haste_pseudo_stats(aura, &imbue.speed_pseudo_stats());
    }
}
