//! Ice Barrier (13033), from Go sim/mage/ice_barrier.go (the fork's patch 187, client hotfix
//! push 112753): the cast shields the mage for its absorb, `warding x (base + 0.1 x Frost spell
//! power)` read as it is cast, and applies Frigid Power. Each aura raises Frost damage in Piercing
//! Ice's old shape while it is up: the SPELLMOD_DAMAGE share on every Frost spell but Frostfire
//! Bolt, the SPELLMOD_DOT share on all of Frostfire Bolt and the rest of the first on its hit.

use crate::{
    classes::mage::masks::{except, ALL},
    core::fight::{Agent, AuraRef, Fight, ModId, ModKind, SCHOOL_INDEX_FROST},
};

/// Go `core.SpellSchoolFrost`.
const FROST: u8 = 16;
/// The shield's absorb effect names every school (Misc 127).
const ALL_SCHOOLS: u8 = 127;

#[derive(Clone, Debug)]
pub(crate) struct IceBarrier {
    pub(crate) shield: AuraRef,
    pub(crate) frigid_power: AuraRef,
    /// The shield's place in `Fight::item_absorbs`.
    absorb: usize,
    absorb_base: f64,
    warding: f64,
    shield_mods: [ModId; 3],
    frigid_mods: [ModId; 3],
}

/// The three modifiers of Go `attachFrostDamageBonus`, in its attach order.
fn frost_damage_bonus<A: Agent>(fight: &mut Fight<A>, hit: f64, dot: f64) -> [ModId; 3] {
    let others = fight.spells_with_class_and_school(&except(ALL, &["frostfire_bolt"]), FROST);
    let damage = fight.register_mod(ModKind::DamageDoneFlat, hit, 0, others);
    let frostfire = fight.spells_with_class(&["frostfire_bolt"]);
    let dot_mod = fight.register_mod(ModKind::DamageDoneFlat, dot, 0, frostfire.clone());
    let direct = fight.register_mod(ModKind::DirectDamageDoneFlat, hit - dot, 0, frostfire);
    [damage, dot_mod, direct]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    aura: &str,
    frigid_power_aura: &str,
    absorb_base: f64,
    warding: f64,
    (hit, dot): (f64, f64),
    (frigid_hit, frigid_dot): (f64, f64),
) -> Result<IceBarrier, String> {
    let shield = fight.player_aura(aura)?;
    let frigid_power = fight.player_aura(frigid_power_aura)?;
    // Go registers Frigid Power and its modifiers first, then the shield and its own.
    let frigid_mods = frost_damage_bonus(fight, frigid_hit, frigid_dot);
    let shield_mods = frost_damage_bonus(fight, hit, dot);
    let absorb = fight.push_class_absorb(shield, ALL_SCHOOLS);
    Ok(IceBarrier {
        shield,
        frigid_power,
        absorb,
        absorb_base,
        warding,
        shield_mods,
        frigid_mods,
    })
}

impl IceBarrier {
    /// Go `ApplyEffects`: the absorb from the Frost spell power now, the shield, then Frigid
    /// Power. The arm64 build fuses the base and the spell power's share.
    pub(crate) fn apply<A: Agent>(&self, fight: &mut Fight<A>) {
        let spell_power =
            fight.player.powers.spell_damage + fight.player_school_damage()[SCHOOL_INDEX_FROST];
        let absorb = self.warding * spell_power.mul_add(0.1, self.absorb_base);
        fight.activate_absorb(self.absorb, absorb);
        fight.activate_aura(self.frigid_power);
    }

    pub(crate) fn on_gain<A: Agent>(&self, fight: &mut Fight<A>, aura: AuraRef) {
        for modifier in self.mods(aura) {
            fight.activate_mod(modifier);
        }
    }

    pub(crate) fn on_expire<A: Agent>(&self, fight: &mut Fight<A>, aura: AuraRef) {
        for modifier in self.mods(aura) {
            fight.deactivate_mod(modifier);
        }
    }

    fn mods(&self, aura: AuraRef) -> [ModId; 3] {
        if aura == self.shield {
            self.shield_mods
        } else {
            self.frigid_mods
        }
    }
}
