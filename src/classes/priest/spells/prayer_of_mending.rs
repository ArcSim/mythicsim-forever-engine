//! Prayer of Mending in MythicSim's healer sims under Forever's rules: the buff goes on the tank
//! and waits there until the tank is hit. Then a charge heals, and the buff moves to another
//! member of the party with a charge less and its duration again (30 seconds a player, as the
//! tooltip states). It ends when the charges run out, when it waits a whole duration on a player
//! nobody hits, or when a new cast replaces it.
//!
//! Who is hit, and when, comes from the party's damage (`prepare::party_damage`): each member's
//! hits arrive at random at its rate, so the wait for the next one is drawn afresh each time.
//! Under the Go reference's rules the heal book heals all five charges at once instead.

use crate::core::{
    fight::{heals::HealBook, Fight, PRIORITY_LOW},
    queue::Handle,
};

use super::super::agent::PriestAgent;

/// The class action that runs a charge.
pub(crate) const MENDING_TAG: u32 = 1;

/// The tank, where a cast puts the buff.
const TANK: usize = 0;

/// The buff running on the party.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MendingRun {
    rank: usize,
    charges: u32,
    duration: i64,
    holder: usize,
    /// The next charge's heal, when the holder is hit before the buff runs out.
    hop: Option<Handle>,
}

/// A cast of rank `rank`: the buff replaces the one running and goes on the tank.
pub(crate) fn cast(
    fight: &mut Fight<PriestAgent>,
    book: &HealBook,
    rank: usize,
    charges: u32,
    duration: i64,
) {
    if let Some(hop) = fight.agent.mending.take().and_then(|run| run.hop) {
        fight.queue.cancel(hop);
    }
    let run = MendingRun {
        rank,
        charges,
        duration,
        holder: TANK,
        hop: None,
    };
    wait(fight, book, run);
}

/// The holder is hit: a charge heals and the buff moves on with what is left.
pub(crate) fn hop(fight: &mut Fight<PriestAgent>, book: &HealBook) {
    let Some(mut run) = fight.agent.mending.take() else {
        return;
    };
    book.mending_heal(fight, run.rank);
    run.charges -= 1;
    if run.charges == 0 {
        return;
    }
    run.holder = book.next_holder(fight, run.holder);
    wait(fight, book, run);
}

/// Waits on the holder for its next hit, when that comes within the buff's duration.
fn wait(fight: &mut Fight<PriestAgent>, book: &HealBook, mut run: MendingRun) {
    let wait = book.mending_wait(fight, run.holder);
    if wait > run.duration {
        return;
    }
    let at = fight.now + wait;
    run.hop = Some(fight.schedule_class_action(at, PRIORITY_LOW, MENDING_TAG));
    fight.agent.mending = Some(run);
}
