package main

import (
	"github.com/wowsims/forever/sim/common/forever"
	"github.com/wowsims/forever/sim/core"
	"github.com/wowsims/forever/sim/core/dbcenums"
	"github.com/wowsims/forever/sim/core/proto"
	"github.com/wowsims/forever/sim/core/spelldata"
)

// common/forever/revelation.go revelationBuffs: each class's buff mask, client 1.60.1.70425, which
// the package keeps unexported. Warriors and rogues have none.
var revelationBuffMasks = map[proto.Class][4]uint32{
	proto.Class_ClassMage:    {547494642, 0, 0, 8},
	proto.Class_ClassPriest:  {161758864, 8486934, 128, 0},
	proto.Class_ClassWarlock: {541161, 8650816, 0, 0},
	proto.Class_ClassDruid:   {4194341, 2, 0, 0},
	proto.Class_ClassHunter:  {10240, 0, 0, 0},
	proto.Class_ClassShaman:  {0x801001c3, 266240, 0, 0},
	proto.Class_ClassPaladin: {0xc0200400, 2097792, 64, 67108864},
}

// revelation.go hasPeriodicPart and ticksByCastingASpell: whether a row carries one of the auras.
func rowHasAura(row *spelldata.Spell, auras ...dbcenums.EffectAuraType) bool {
	for i := range row.Effects {
		for _, aura := range auras {
			if row.Effects[i].Aura == aura {
				return true
			}
		}
	}
	return false
}

// revelation.go revelationCanTriggerFrom: whether a hit, or a tick, of the spell can trigger it.
func revelationCanTriggerFrom(spell *core.Spell, periodic bool) bool {
	row := spelldata.Find(spell.ActionID.SpellID)
	if periodic {
		return spell.AOEDot() != nil && rowHasAura(row, dbcenums.A_PERIODIC_TRIGGER_SPELL, dbcenums.A_PERIODIC_DUMMY)
	}
	periodicPart := rowHasAura(row, dbcenums.A_PERIODIC_DAMAGE, dbcenums.A_PERIODIC_LEECH, dbcenums.A_PERIODIC_HEAL)
	if spell.ProcMask.Matches(core.ProcMaskSpellHealing) {
		return !periodicPart
	}
	return spell.DefenseType == core.DefenseTypeMagic && !periodicPart
}

// common/forever/revelation.go applyRevelation (mythicsim patches 157 and 170): two triggers, one on
// hits and heals and one on ticks, roll the chance on each eligible landed hit that does not crit and
// deals something, and a batch window later activate the class buff, which gives the spells its mask
// names 100% more crit. The next cast of one of them that is no channel or proc spends it, and the end
// of a channel it names ends it. Exported once the enchant registered its auras.
func revelationEffect(character *core.Character, unrepresented *[]string) map[string]any {
	trigger := character.GetAura("Enchant Weapon - Revelation")
	ticks := character.GetAura("Enchant Weapon - Revelation (ticks)")
	buff := character.GetAura("Revelation")
	if trigger == nil || ticks == nil || buff == nil {
		return nil
	}
	if !forever.RevelationChanceFallsWithCrit {
		*unrepresented = append(*unrepresented, "Revelation's chance does not fall with crit")
	}
	mask := core.ClassFlags{Family: core.ClassSpellFamilies[character.Class], Mask: revelationBuffMasks[character.Class]}
	affected, consuming, channels := []int{}, []int{}, []int{}
	for i, spell := range character.Spellbook {
		flags := spell.ClassFlags
		if flags.IsZero() {
			flags = core.ClientClassFlags(spell.ActionID.SpellID)
		}
		if spell.ActionID.SpellID == 0 || !mask.Matches(flags) {
			continue
		}
		affected = append(affected, i)
		if spell.Flags.Matches(core.SpellFlagChanneled) {
			channels = append(channels, i)
		}
		if !spell.Flags.Matches(core.SpellFlagChanneled | core.SpellFlagProc) {
			consuming = append(consuming, i)
		}
	}
	listener := core.ProcTrigger{
		ProcMask:         core.ProcMaskSpellDamage | core.ProcMaskSpellDamageProc | core.ProcMaskSpellHealing,
		CanProcFromProcs: true,
	}
	eligible := func(periodic bool) []int {
		out := []int{}
		for _, i := range procTriggerSpells(character, listener) {
			if revelationCanTriggerFrom(character.Spellbook[i], periodic) {
				out = append(out, i)
			}
		}
		return out
	}
	return map[string]any{
		"kind": "revelation", "trigger_aura": trigger.Label, "ticks_trigger_aura": ticks.Label, "aura": buff.Label,
		"hit_spells": eligible(false), "tick_spells": eligible(true), "affected_spells": affected,
		"consuming_spells": consuming, "channel_spells": channels, "bonus_crit_percent": 100.0,
		"chance_at_no_crit": forever.RevelationChanceAtNoCrit, "delay_ns": nanos(core.SpellBatchWindow),
	}
}
