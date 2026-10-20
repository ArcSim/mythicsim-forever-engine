package main

import (
	"fmt"

	"github.com/wowsims/forever/sim/core"
)

// consumes_forever_potions.go: the Merchant's Favor potions whose effect is not a stat buff
// (mythicsim patch 167). Their amounts are Go literals.
var foreverPotions = map[int32]bool{
	core.DragonfirePotionItemID:       true,
	core.CausticSmogPotionItemID:      true,
	core.ElementalPurgingPotionItemID: true,
}

// The effect of one of the potions: the spell its cast casts, or the aura it activates.
func foreverPotionEffect(character *core.Character, item int32, unrepresented *[]string) map[string]any {
	spellOf := func(id int32) int {
		spell := character.GetSpell(core.ActionID{SpellID: id})
		if spell == nil {
			*unrepresented = append(*unrepresented, fmt.Sprintf("potion %d has no spell %d", item, id))
			return -1
		}
		return spellPosition(character, spell)
	}
	switch item {
	case core.DragonfirePotionItemID:
		// newDragonfirePotionSpell: 280 Fire on the magic hit and crit table on every target, then
		// on each it lands on 35 every 2 sec for 8 sec.
		return map[string]any{
			"kind": "dragonfire_potion", "item_id": item, "spell": spellOf(1252554),
			"direct_damage": 280.0, "tick_damage": 35.0,
		}
	case core.CausticSmogPotionItemID:
		// newCausticSmogPotionSpell: a magic hit roll on every target, then on each it lands on 33
		// Nature every sec for 12 sec, each tick rolled +-10% under the potion's name.
		return map[string]any{
			"kind": "caustic_smog_potion", "item_id": item, "spell": spellOf(1252543),
			"tick_damage": 33.0, "tick_variance": 0.1, "rng_label": "Caustic Smog Potion",
		}
	}
	// newElementalPurgingAura: 105 spell damage against Elementals while the aura holds.
	label := "Potion of Elemental Purging"
	if character.GetAura(label) == nil {
		*unrepresented = append(*unrepresented, fmt.Sprintf("potion %d has no aura", item))
	}
	return map[string]any{
		"kind": "elemental_purging_potion", "item_id": item, "aura": label,
		"spell_damage": 105.0, "mob_type": "MobTypeElemental",
	}
}
