//! The Restoration Druid's heals and heal talents, which Rust alone sims (see `prepare::heals`):
//! every rank of Healing Touch, Rejuvenation and Regrowth, and the talents' Wild Growth and
//! Swiftmend, from the client rows of build 1.60.1.70425.
//!
//! Wild Growth's ticks land on the party of five. Swiftmend consumes a Rejuvenation or Regrowth
//! up on the player and heals what all of that heal over time's ticks would, as Forever's
//! Swiftmend does (`classes::druid::spells::swiftmend`). Tranquility, a channel on the party, and
//! Nature's Swiftness on a heal are not modelled yet.

use crate::prepare::heals::{apply_heal_talent, register_heal, HealShape};
use crate::prepare::sim::Sim;
use crate::prepare::spell::SpellFlag;
use crate::prepare::spell_mod::{SpellModConfig, SpellModType};
use crate::prepare::spelldata::store::must_find;

use super::masks;
use super::Druid;

pub(crate) const HEALING_TOUCH: &[i32] = &[
    5185, 5186, 5187, 5188, 5189, 6778, 8903, 9758, 9888, 9889, 25297,
];
pub(crate) const REJUVENATION: &[i32] = &[
    774, 1058, 1430, 2090, 2091, 3627, 8910, 9839, 9840, 9841, 25299,
];
pub(crate) const REGROWTH: &[i32] = &[8936, 8938, 8939, 8940, 8941, 9750, 9856, 9857, 9858];
/// Wild Growth, a Forever talent: a heal over time on the target's party.
const WILD_GROWTH: &[i32] = &[408120, 1238214, 1238215];
const SWIFTMEND: i32 = 18562;
const PARTY: u32 = 5;

/// The heal talents, as (talent, client row, ranks): Gift of Nature's direct and periodic
/// healing, Tranquil Spirit's cost, Improved Rejuvenation, Improved Regrowth's crit, Naturalist's
/// Healing Touch cast time and Gift of the Earthmother's global cooldown.
const HEAL_TALENTS: &[(&str, i32, i32)] = &[
    ("gift_of_nature", 17104, 5),
    ("tranquil_spirit", 24968, 5),
    ("improved_rejuvenation", 17111, 3),
    ("improved_regrowth", 17074, 5),
    ("naturalist", 17069, 5),
    ("gift_of_the_earthmother", 414673, 1),
];

/// Nature's Splendor (1223083) lengthens Rejuvenation by 3 seconds and Regrowth by 6, one and
/// two of their 3 second ticks.
const NATURES_SPLENDOR_TICKS: &[(i64, i32)] = &[(masks::REJUVENATION, 1), (masks::REGROWTH, 2)];

impl Druid {
    /// Registers every heal rank the druid knows.
    pub(super) fn register_heals(&mut self, sim: &mut Sim) {
        let unit = self.unit;
        let mut ranks = Vec::new();
        let mut each =
            |sim: &mut Sim, ids: &[i32], mask: i64, shape: HealShape, shared_cd: bool| {
                let timer = shared_cd.then(|| sim.new_timer(unit));
                for &id in ids {
                    ranks.push(register_heal(sim, unit, must_find(id), mask, shape, timer));
                }
            };
        each(
            sim,
            HEALING_TOUCH,
            masks::HEALING_TOUCH,
            HealShape::SINGLE,
            false,
        );
        each(
            sim,
            REJUVENATION,
            masks::REJUVENATION,
            HealShape::SINGLE,
            false,
        );
        each(sim, REGROWTH, masks::REGROWTH, HealShape::SINGLE, false);
        if self.talents.bool("wild_growth") {
            each(
                sim,
                WILD_GROWTH,
                masks::WILD_GROWTH,
                HealShape::party(PARTY),
                true,
            );
        }
        if self.talents.bool("swiftmend") {
            each(
                sim,
                &[SWIFTMEND],
                masks::SWIFTMEND,
                HealShape::SINGLE,
                false,
            );
        }
        self.heals = ranks;
    }

    /// Applies the heal talents the druid took.
    pub(super) fn apply_heal_talents(&self, sim: &mut Sim) {
        for &(name, spell_id, max_ranks) in HEAL_TALENTS {
            let rank = crate::prepare::heals::talent_rank(&self.talents, name);
            apply_heal_talent(sim, self.unit, spell_id, max_ranks, rank);
        }
        if self.tal.natures_splendor {
            for &(mask, ticks) in NATURES_SPLENDOR_TICKS {
                sim.add_static_mod(
                    self.unit,
                    SpellModConfig {
                        class_mask: mask,
                        spell_flag: SpellFlag::HELPFUL,
                        kind: SpellModType::DotNumberOfTicksFlat,
                        int_value: ticks,
                        ..SpellModConfig::default()
                    },
                );
            }
        }
    }
}
