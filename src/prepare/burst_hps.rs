//! Burst healing per second for MythicSim's healer sims under Forever's rules: beside healing
//! per second over the whole fight, which the healer's mana spreads thin (sustained), each fight
//! reports the most it heals in any ten seconds, over ten seconds (burst), as the moments a
//! group takes the most damage ask (`core::fight::burst`). Ten seconds is a judgement, not a
//! measurement: long enough for a few casts and a heal over time's ticks, short enough that
//! mana does not decide it.

use serde_json::{json, Value};

use crate::core::time::NS_PER_SECOND;

/// The burst window.
pub(crate) const WINDOW: i64 = 10 * NS_PER_SECOND;

/// The player's `burst_window`.
pub(crate) fn export() -> Value {
    json!(WINDOW)
}
