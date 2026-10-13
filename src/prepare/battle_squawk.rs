//! Go sim/core/battle_squawk.go: the Battle Squawk stacks the party's engineers give the player
//! (the fork's patch 127, `Party.battle_squawks`).

use std::rc::Rc;

use crate::contracts::prepared_v2::ActionId;

use super::sim::{AuraConfig, Sim, UnitId, SECOND};

/// Go `MaxBattleSquawks`: the most Battle Squawk stacks a party can assume, one per party member.
const MAX_BATTLE_SQUAWKS: i32 = 5;

/// Each stack's melee attack speed, 5%.
const PER_STACK: f64 = 1.05;

/// Go `math.Pow` for an integer exponent: the product of the base's mantissa taken at every set bit of
/// the exponent, scaled by powers of two at the end, so each rounding is the one Go makes.
fn go_pow(x: f64, exponent: i32) -> f64 {
    if exponent == 0 || x == 1.0 {
        return 1.0;
    }
    if exponent == 1 {
        return x;
    }
    let (mut mantissa, mut exp) = frexp(x);
    let mut product = 1.0;
    let mut product_exp = 0;
    let mut bits = i64::from(exponent.unsigned_abs());
    while bits != 0 {
        if bits & 1 == 1 {
            product *= mantissa;
            product_exp += exp;
        }
        mantissa *= mantissa;
        exp <<= 1;
        if mantissa < 0.5 {
            mantissa += mantissa;
            exp -= 1;
        }
        bits >>= 1;
    }
    if exponent < 0 {
        product = 1.0 / product;
        product_exp = -product_exp;
    }
    ldexp(product, product_exp)
}

/// Go `math.Frexp` of a normal number: the fraction in [0.5, 1) and the power of two it scales by.
fn frexp(x: f64) -> (f64, i32) {
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = f64::from_bits((bits & !(0x7ff << 52)) | (1022 << 52));
    (fraction, exponent - 1022)
}

/// Go `math.Ldexp` for a result in the normal range.
fn ldexp(fraction: f64, exp: i32) -> f64 {
    fraction * 2f64.powi(exp)
}

impl Sim {
    /// Go `registerBattleSquawk`: the aura an engineer's Gnomish Battle Chicken gives its party,
    /// 5% melee attack speed a stack, taken as cast at the pull.
    pub(crate) fn register_battle_squawk(&mut self, unit: UnitId, stacks: i32) {
        if stacks <= 0 {
            return;
        }
        let stacks = stacks.min(MAX_BATTLE_SQUAWKS);
        self.get_or_register_aura(
            unit,
            AuraConfig {
                label: "Battle Squawk".to_string(),
                action_id: Some(ActionId::spell(23060)),
                duration: 4 * 60 * SECOND,
                max_stacks: MAX_BATTLE_SQUAWKS,
                on_reset: Some(Rc::new(move |sim: &mut Sim, aura| {
                    sim.activate(aura);
                    sim.set_stacks(aura, stacks);
                })),
                on_stacks_change: Some(Rc::new(move |sim: &mut Sim, aura, old, new| {
                    let unit = sim.aura(aura).unit;
                    sim.multiply_melee_speed(unit, go_pow(PER_STACK, new - old));
                })),
                ..AuraConfig::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_pow_multiplies_the_normalised_mantissa_like_go() {
        // Go's math.Pow(1.05, n), n = 1..5, from the reference engine.
        let expected = [
            1.05,
            1.1025,
            1.1576250000000001,
            1.21550625,
            1.2762815625000001,
        ];
        for (n, want) in (1..=5).zip(expected) {
            assert_eq!(go_pow(PER_STACK, n), want, "1.05 to the {n}");
        }
        assert_eq!(go_pow(PER_STACK, 0), 1.0);
    }

    #[test]
    fn frexp_and_ldexp_round_trip() {
        for x in [1.05, 0.5, 3.0, 1234.5678] {
            let (fraction, exp) = frexp(x);
            assert!((0.5..1.0).contains(&fraction));
            assert_eq!(ldexp(fraction, exp), x);
        }
    }
}
