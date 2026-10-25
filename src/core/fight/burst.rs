//! Burst healing in MythicSim's healer sims: the most healing the player lands in any window of
//! the burst length in a fight, over that length. Healing per second spreads the healer's mana
//! over the whole fight (sustained); burst is how hard it heals while it has mana to spend, as in
//! the moments a group takes the most damage. It counts what healing per second counts: each heal
//! and shield on the player, less its overhealing share.
//!
//! The best window ends on a heal, so keeping the heals of the last window and their sum as each
//! one lands finds it exactly.

use std::collections::VecDeque;

/// The heals of the window running and the best window so far in this fight.
#[derive(Clone, Debug)]
pub(crate) struct BurstWindow {
    /// The window's length, in nanoseconds.
    pub(crate) length: i64,
    heals: VecDeque<(i64, f64)>,
    sum: f64,
    pub(crate) best: f64,
}

impl BurstWindow {
    pub(crate) fn new(length: i64) -> Self {
        BurstWindow {
            length,
            heals: VecDeque::new(),
            sum: 0.0,
            best: 0.0,
        }
    }

    /// A heal of `amount` at `now`: the window becomes the heals since `now` less its length.
    pub(crate) fn add(&mut self, now: i64, amount: f64) {
        self.heals.push_back((now, amount));
        self.sum += amount;
        while let Some(&(at, old)) = self.heals.front() {
            if at > now - self.length {
                break;
            }
            self.heals.pop_front();
            self.sum -= old;
        }
        self.best = self.best.max(self.sum);
    }

    /// A new fight.
    pub(crate) fn reset(&mut self) {
        self.heals.clear();
        self.sum = 0.0;
        self.best = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: i64 = 1_000_000_000;

    #[test]
    fn the_best_window_is_the_most_healing_inside_its_length() {
        let mut window = BurstWindow::new(10 * SECOND);
        // 100 at 0 s, 300 at 5 s, 200 at 10 s: 0 s falls out at 10 s, so 500.
        window.add(0, 100.0);
        window.add(5 * SECOND, 300.0);
        window.add(10 * SECOND, 200.0);
        assert_eq!(window.best, 500.0);
        // 50 at 14 s: 300 + 200 + 50 = 550.
        window.add(14 * SECOND, 50.0);
        assert_eq!(window.best, 550.0);
        // 10 at 30 s stands alone; the best stays.
        window.add(30 * SECOND, 10.0);
        assert_eq!(window.best, 550.0);
        window.reset();
        window.add(SECOND, 7.0);
        assert_eq!(window.best, 7.0);
    }
}
