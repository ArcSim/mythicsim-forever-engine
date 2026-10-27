# Forever gap requests

Production Quick Sim requests that the Rust engine refused between 5 and 27 October 2026, which then
ran in the Go engine or needed its exporter to prepare them. The character names are replaced and the iterations cut to 50. Each one
stands for a group of the refused requests and is read by
[tests/forever_gaps.rs](../../tests/forever_gaps.rs), which prepares it under Forever's rules,
checks the gate passes and runs it.

| Request | What Rust lacked |
|---|---|
| `protection-paladin-seal-of-light` | Seal of Light and Judgement of Light on a tank, the party's Retribution Aura striking back |
| `protection-paladin-retribution-aura` | the paladin's own Retribution Aura cast (rank 2, moved to 7 sec before the pull so it is not on the GCD) |
| `protection-warrior-thunder-clap` | Thunder Clap beside a permanent slow in its category |
| `enhancement-shaman-windfury-swap` | Windfury Weapon on a swap weapon against the party's Windfury Totem |
| `retribution-paladin-swap-blade-of-hanna` | a swap onto Blade of Hanna, a weapon with its own proc |
| `retribution-paladin-swap-sulfuras` | a swap onto Sulfuras, Hand of Ragnaros |
| `shadow-priest-vampiric-embrace` | Vampiric Embrace |
| `shadow-priest-prepull-sequence` | a strict sequence among the prepull actions, its first step on a time condition |
| `marksmanship-hunter-sequence-conditions` | sequence steps with conditions |
| `enhancement-shaman-many-stat-auras` | fourteen stat auras |
| `retribution-paladin-spell-value-fold` | a spell value condition the Rust preparation could not fold |
| `demonology-warlock-pet-values`, `demonology-warlock-pet-aura-time` | a demon's mana percent and aura time |
| `combat-rogue-aura-value-without-aura` | an aura time without an aura |
| `enhancement-shaman-stormcallers-garb` | Stormcaller's Garb (3) |
| `hunter-bear-swipe-three-targets` | the Bear's Swipe on three targets |
| `enhancement-shaman-frostbrand-swap` | Frostbrand Weapon on the swap weapon |
| `feral-druid-swap-heart-of-wyrmthalak` | a swap with Heart of Wyrmthalak's proc manager |
| `enhancement-shaman-swap-fiery-weapon` | a swap weapon with Fiery Weapon |
| `protection-paladin-judgement-of-light-three-targets` | the raid's Judgement of Light on three targets |
| `demonology-warlock-unsummoned-pet-mana` | the mana percent of a demon the fight never summons |
| `combat-rogue-darkmantle-armor` | Darkmantle Armor (4)'s energy |
| `combat-rogue-darkmantle-riposte` | Riposte without a tank |
| `dps-warrior-pummel` | Pummel |
| `dps-warrior-battlegear-of-heroism` | Battlegear of Heroism (4) |
| `elemental-shaman-the-elements` | The Furious Storm for a spec other than Restoration |
| `demonology-warlock-mob-type-bonus` | a bonus against demons on a target of another type |
| `combat-rogue-battle-squawk` | a fight that outlasts Battle Squawk |
| `demonology-warlock-assigned-curse` | the assigned curse at all targets |
| `retribution-paladin-swap-thunderfury` | a swap onto Thunderfury |
| `retribution-paladin-swap-crit-taken` | a swap that changes the crit the player takes |
| `retribution-paladin-swap-runeblade` | a swap onto the Runeblade of Baron Rivendare |
| `hunter-gorilla-thunderstomp` | the Gorilla's Thunderstomp on five targets |
| `hunter-prepull-group-move` | a prepull group reference that moves to a starting range |
| `protection-warrior-fire-hardened-hauberk` | a rage gain on use |
| `demonology-warlock-orb-of-soranruk` | a heal over time on use |
| `demonology-warlock-sequence-other-targets` | sequence steps at the next target |
