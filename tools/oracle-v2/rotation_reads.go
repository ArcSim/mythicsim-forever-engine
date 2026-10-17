// Rotation reads: what the rotation's values read of the simulation that the rest of the export
// leaves out, written only for a rotation that holds such a value.
package main

import (
	"sort"

	"github.com/wowsims/forever/sim/core"
	"github.com/wowsims/forever/sim/core/proto"
	googleProto "google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

// RotationReads holds, for the values a rotation names, what they read.
type RotationReads struct {
	// apl_values_consumables.go: the potion and conjured item the consumables select.
	PotionID   *int32 `json:"potion_id,omitempty"`
	ConjuredID *int32 `json:"conjured_id,omitempty"`
	// apl_values_boss.go: the spells a target knows that a boss value names, and whether each
	// has a cooldown, which a fight without a target AI never starts.
	TargetSpells []RotationTargetSpell `json:"target_spells,omitempty"`
	// apl_values_misc.go APLValueMultipleCdUsages: the related self buff of each spell the value
	// names that has one, and whether the spell has a cooldown.
	SelfBuffs []RotationSelfBuff `json:"self_buffs,omitempty"`
}

// RotationTargetSpell is a spell the first target knows, which its identical copies share.
type RotationTargetSpell struct {
	Spell       ActionID `json:"spell"`
	HasCooldown bool     `json:"has_cooldown,omitempty"`
}

// RotationSelfBuff is a spell's RelatedSelfBuff.
type RotationSelfBuff struct {
	Spell       ActionID `json:"spell"`
	Aura        string   `json:"aura"`
	HasCooldown bool     `json:"has_cooldown,omitempty"`
}

// rotationValues lists every value of the rotation, depth first, as messages.
func rotationValues(rotation *proto.APLRotation) []googleProto.Message {
	var values []googleProto.Message
	var visit func(message protoreflect.Message)
	visit = func(message protoreflect.Message) {
		values = append(values, message.Interface())
		message.Range(func(field protoreflect.FieldDescriptor, value protoreflect.Value) bool {
			switch {
			case field.IsList() && field.Message() != nil:
				for i := 0; i < value.List().Len(); i++ {
					visit(value.List().Get(i).Message())
				}
			case field.IsMap():
				value.Map().Range(func(_ protoreflect.MapKey, entry protoreflect.Value) bool {
					if field.MapValue().Message() != nil {
						visit(entry.Message())
					}
					return true
				})
			case field.Message() != nil:
				visit(value.Message())
			}
			return true
		})
	}
	if rotation != nil {
		visit(rotation.ProtoReflect())
	}
	return values
}

// targetOfBossValue is GetTargetUnit of a boss value's target unit: a target for no reference, the
// current, next, previous or an indexed one, nil for the player or a set, which read no target.
func targetOfBossValue(ref *proto.UnitReference, targets []*core.Unit) *core.Unit {
	if len(targets) == 0 {
		return nil
	}
	switch ref.GetType() {
	case proto.UnitReference_Unknown, proto.UnitReference_CurrentTarget,
		proto.UnitReference_NextTarget, proto.UnitReference_PreviousTarget:
		return targets[0]
	case proto.UnitReference_Target:
		if int(ref.GetIndex()) < len(targets) {
			return targets[ref.GetIndex()]
		}
	}
	return nil
}

func lessActionID(a, b ActionID) bool {
	if a.SpellID != b.SpellID {
		return a.SpellID < b.SpellID
	}
	if a.ItemID != b.ItemID {
		return a.ItemID < b.ItemID
	}
	if a.OtherID != b.OtherID {
		return a.OtherID < b.OtherID
	}
	return a.Tag < b.Tag
}

// rotationReads collects what the rotation's values read, or nil when none of them reads anything
// the export leaves out.
func rotationReads(request *proto.RaidSimRequest, simulation *core.Simulation, character *core.Character) *RotationReads {
	player := request.Raid.Parties[0].Players[0]
	reads := &RotationReads{}
	used := false
	seenTarget := map[core.ActionID]bool{}
	seenBuff := map[core.ActionID]bool{}
	selfBuff := func(id *proto.ActionID) {
		if id == nil {
			return
		}
		spell := aplSpell(&character.Unit, core.ProtoToActionID(id))
		if spell == nil || spell.RelatedSelfBuff == nil || seenBuff[spell.ActionID] {
			return
		}
		seenBuff[spell.ActionID] = true
		reads.SelfBuffs = append(reads.SelfBuffs, RotationSelfBuff{Spell: *actionIDOrEmpty(spell.ActionID),
			Aura: spell.RelatedSelfBuff.Label, HasCooldown: spell.CD.Timer != nil})
		used = true
	}
	targetSpell := func(id *proto.ActionID, ref *proto.UnitReference) {
		target := targetOfBossValue(ref, simulation.Encounter.AllTargetUnits)
		if id == nil || target == nil {
			return
		}
		actionID := core.ProtoToActionID(id)
		spell := target.GetSpell(actionID)
		if spell == nil || seenTarget[actionID] {
			return
		}
		seenTarget[actionID] = true
		reads.TargetSpells = append(reads.TargetSpells, RotationTargetSpell{Spell: *actionIDOrEmpty(actionID),
			HasCooldown: spell.CD.Timer != nil || spell.SharedCD.Timer != nil})
		used = true
	}
	for _, value := range rotationValues(player.GetRotation()) {
		switch v := value.(type) {
		case *proto.APLValueSelectedPotion:
			id := character.Consumables.GetPotId()
			reads.PotionID = &id
			used = true
		case *proto.APLValueSelectedConjured:
			id := character.Consumables.GetConjuredId()
			reads.ConjuredID = &id
			used = true
		case *proto.APLValueBossSpellIsCasting:
			targetSpell(v.GetSpellId(), v.GetTargetUnit())
		case *proto.APLValueBossSpellTimeToReady:
			targetSpell(v.GetSpellId(), v.GetTargetUnit())
		case *proto.APLValueMultipleCdUsages:
			selfBuff(v.GetBaseSpellId())
			selfBuff(v.GetTargetSpellId())
		}
	}
	if !used {
		return nil
	}
	sort.Slice(reads.TargetSpells, func(i, j int) bool {
		return lessActionID(reads.TargetSpells[i].Spell, reads.TargetSpells[j].Spell)
	})
	sort.Slice(reads.SelfBuffs, func(i, j int) bool { return lessActionID(reads.SelfBuffs[i].Spell, reads.SelfBuffs[j].Spell) })
	return reads
}
