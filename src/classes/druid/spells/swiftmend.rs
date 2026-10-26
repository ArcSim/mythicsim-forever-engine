//! Swiftmend (18562), a Restoration talent in MythicSim's healer sims (Rust only): it consumes a
//! Rejuvenation or Regrowth up on the player and heals for what all of that heal over time's
//! ticks would, as Forever's tooltip states ("the full duration"), with the healing crit roll.
//! Rejuvenation goes first, then Regrowth, the highest rank of each first.

use crate::core::fight::{heals::HealBook, Fight, SpellId};

use super::super::{agent::DruidAgent, prepare::heals};

/// The heals over time Swiftmend consumes, in the order it looks for them.
pub(crate) const CONSUMES: [&[i32]; 2] = [heals::REJUVENATION, heals::REGROWTH];

/// A cast: the first heal over time up, consumed, then its whole heal.
pub(crate) fn cast(
    fight: &mut Fight<DruidAgent>,
    book: &HealBook,
    ranks: &[usize],
    spell: SpellId,
) {
    // Idol of Synthesis counts the heals over time up before Swiftmend consumes one.
    if let Some(rank) = book.rank_of(spell) {
        book.cut_cooldown(fight, rank);
    }
    let Some((dot, amount)) = book.hot_up(fight, ranks) else {
        return;
    };
    let aura = fight.dots[dot].aura;
    fight.deactivate_aura(aura);
    book.heal_for(fight, spell, amount);
}
