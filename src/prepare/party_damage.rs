//! The damage a healer's party takes in MythicSim's healer sims under Forever's rules, for the
//! heals whose reach follows it: Prayer of Mending, whose charges move only when the player
//! holding them is hit, and Chain Heal, which jumps only to players who need healing.
//!
//! These rates are an assumption, not a measurement. Forever's level 30 beta logs on
//! foreverlogs.gg hold neither heal (Prayer of Mending and Chain Heal come later), so nothing
//! calibrates them yet: recalibrate once the logs reach their levels. A party of five, the tank
//! first: the tank is hit about once a second (a boss's swing and what else it holds), each of
//! the other four about once every five seconds (cleaves, area damage). A member hit within the
//! last four seconds still needs healing.
//!
//! The rest of a healer's heals keep landing on a player who needs them, less the share real
//! healers overheal (`prepare::overheal`), as before.

use serde_json::{json, Value};

use crate::core::time::NS_PER_SECOND;

/// Hits a second on each member of the party, the tank first.
pub(crate) const HIT_RATES: [f64; 5] = [1.0, 0.2, 0.2, 0.2, 0.2];

/// How long after a hit a member still needs healing.
pub(crate) const INJURED_WINDOW: i64 = 4 * NS_PER_SECOND;

/// The `party` of a `heals` effect.
pub(crate) fn export() -> Value {
    json!({"hit_rates": HIT_RATES, "injured_window": INJURED_WINDOW})
}
