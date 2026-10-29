//! Winter's Chill (talent 11180, aura 12579), from Go sim/mage/talents_frost.go
//! `registerWinterChill`: a landed Frost spell can add a stack, and each stack makes the mage's
//! Cone of Cold deal more damage (client 70425).
//!
//! Forever's rules follow client 1.60.1.70630 (`cone_of_cold_consumes`): "Gives your Frostbolt,
//! Frost Blast, and Fingers of Frost empowered Ice Lance spells a chance to apply Winter's Chill,
//! increasing the target's damage taken from your next Cone of Cold by 5% and it gains an
//! additional 7% of your spell damage, stacking up to 5 times". So only those three add stacks
//! (11180 SpellEffect row, ProcCharges 1), each stack adds 5% damage and 7% of the mage's spell
//! damage to the Cone of Cold, and the Cone of Cold uses every stack up (12579 ProcTypeMask
//! 131072, a hit of the caster's harmful spell). The reference keeps the 70425 rule.

use crate::core::fight::{Agent, AuraRef, Fight, ModId, ModKind, Side, SpellId, SpellResult};

/// The spells that apply Winter's Chill under client 70630's rule; the Ice Lance only when
/// Fingers of Frost empowers it, which the caller tells.
const APPLIED_BY: &[&str] = &["frostbolt", "frost_blast"];

#[derive(Clone, Debug)]
pub(crate) struct WintersChill {
    pub(crate) aura: AuraRef,
    pub(crate) trigger: AuraRef,
    proc_chance: f64,
    damage_per_stack: f64,
    damage_mod: ModId,
    /// Client 70630: the share of spell damage each stack adds to the next Cone of Cold.
    spell_power_per_stack: f64,
    /// Client 70630: the rule above. False keeps the 70425 one.
    cone_of_cold_consumes: bool,
}

/// Bind the exported auras and register Go's dynamic damage modifier.
pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    aura: &str,
    trigger: &str,
    proc_chance: f64,
    damage_per_stack: f64,
    (spell_power_per_stack, cone_of_cold_consumes): (f64, bool),
) -> Result<WintersChill, String> {
    let find = |label: &str| {
        fight.trackers[Side::Player.index()]
            .find(label)
            .map(|index| AuraRef {
                side: Side::Player,
                index,
            })
            .ok_or_else(|| format!("Winter's Chill aura {label} is not registered"))
    };
    let (aura, trigger) = (find(aura)?, find(trigger)?);
    let affected = fight.spells_with_class(&["cone_of_cold"]);
    let damage_mod = fight.register_mod(ModKind::DamageDoneFlat, 0.0, 0, affected);
    Ok(WintersChill {
        aura,
        trigger,
        proc_chance,
        damage_per_stack,
        damage_mod,
        spell_power_per_stack,
        cone_of_cold_consumes,
    })
}

impl WintersChill {
    pub(crate) fn on_gain<A: Agent>(&self, fight: &mut Fight<A>) {
        fight.activate_mod(self.damage_mod);
    }

    pub(crate) fn on_expire<A: Agent>(&self, fight: &mut Fight<A>) {
        fight.deactivate_mod(self.damage_mod);
    }

    pub(crate) fn on_stacks_change<A: Agent>(&self, fight: &mut Fight<A>, stacks: i32) {
        fight.update_mod_value(self.damage_mod, self.damage_per_stack * f64::from(stacks));
    }

    /// The "Winters Chill Talent" proc trigger: spell damage, landed, Frost school; under
    /// client 70630's rule a Frostbolt or a Frost Blast alone.
    pub(crate) fn on_spell_hit_dealt<A: Agent>(
        &self,
        fight: &mut Fight<A>,
        spell: SpellId,
        result: &SpellResult,
    ) {
        let state = &fight.spells[spell];
        if state.flags.proc
            || !state.proc_spell_damage
            || !result.landed()
            || state.school & 16 == 0
        {
            return;
        }
        if self.cone_of_cold_consumes
            && !state
                .class_spell
                .as_deref()
                .is_some_and(|class| APPLIED_BY.contains(&class))
        {
            return;
        }
        self.roll_stack(fight);
    }

    /// A Fingers of Frost empowered Ice Lance applies Winter's Chill (client 70630). Like the
    /// charges the Ice Lance spends, the roll is taken as it is cast, on the hit already rolled,
    /// since the Fingers of Frost charge is gone by the time the lance lands.
    pub(crate) fn on_empowered_ice_lance<A: Agent>(
        &self,
        fight: &mut Fight<A>,
        result: &SpellResult,
    ) {
        if self.cone_of_cold_consumes && result.landed() {
            self.roll_stack(fight);
        }
    }

    fn roll_stack<A: Agent>(&self, fight: &mut Fight<A>) {
        if self.proc_chance != 1.0 && fight.random_for_aura(self.trigger) > self.proc_chance {
            return;
        }
        fight.activate_aura(self.aura);
        fight.add_stack(self.aura);
    }

    /// The share of the mage's spell damage the stacks add to a Cone of Cold cast now; zero
    /// under the 70425 rule.
    pub(crate) fn cone_of_cold_coefficient<A: Agent>(&self, fight: &Fight<A>) -> f64 {
        let aura = fight.aura(self.aura);
        if !aura.active {
            return 0.0;
        }
        self.spell_power_per_stack * f64::from(aura.stacks)
    }

    /// The Cone of Cold's cast completing (after its damage) uses the stacks up.
    pub(crate) fn on_cast_complete<A: Agent>(&self, fight: &mut Fight<A>, spell: SpellId) {
        if self.cone_of_cold_consumes
            && fight.spells[spell].class_spell.as_deref() == Some("cone_of_cold")
            && fight.aura(self.aura).active
        {
            fight.deactivate_aura(self.aura);
        }
    }
}
