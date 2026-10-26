# MythicSim Forever Engine

A Rust simulation engine for WoW Forever. [MythicSim](https://mythicsim.com/wow-forever/sim)
runs Forever Quick Sims on it, and the Go engine for the rest; [Coverage](#coverage) lists
which is which.

The engine began as an exact port of [MythicSim's Go fork](https://github.com/sage3648/mythicsim-forever-engine-go)
of [wowsims Forever](https://github.com/ElliotWood/Forever): event order, random numbers and
float math match, so a sim the Go engine covers gives the same result and the same combat log
in either engine. Rust is 1.3x to 2x faster. Since October 2026 new Forever work happens here
only: the Go fork no longer takes patches and its frozen goldens guard what it covered, and
mechanics Go never had, starting with the healer sims, are built and checked in Rust alone
(see [Rust-only mechanics](#rust-only-mechanics)).

## Quick start

Install stable Rust, then run:

```sh
git clone https://github.com/sage3648/mythicsim-forever-engine.git
cd mythicsim-forever-engine
cargo test --locked
cargo run --locked --release -- check --infile fixtures/mage/prepared-v2/frost-reference.prepared.json
cargo run --locked --release -- sim --infile fixtures/mage/prepared-v2/frost-reference.prepared.json
```

`check` lists anything in an input that Rust does not support. `sim` runs the fight
and prints the same JSON result that Go prints. You do not need Go to build or test
the engine.

## How it works

1. Rust prepares the character from the application's request: gear, stats, auras,
   spells and rotation ([prepared v2](docs/prepared-v2.md),
   [Rust preparation](docs/rust-preparation.md)).
2. A coverage gate checks the prepared input and refuses anything Rust cannot simulate
   exactly, with a reason for each refusal.
3. Rust runs the fight.

[`tools/route.py`](docs/routing.md) is what the MythicSim worker calls. It runs a request in
Rust, or tells the worker to run it in Go and says why.

## Coverage

MythicSim's Forever Quick Sims run in Rust, for every class and spec it offers: the Druid,
Hunter, Mage, Paladin, Priest, Rogue, Shaman, Warlock and Warrior damage and tank specs, with
their talents, gear, buffs, consumables, pets, tanking and 2 to 5 targets. Holy Paladin heal
sims match Go too, under the reference's rules: heals cast on the player and counted in full, as
Go counts them. Production prepares them with Forever's rules, which leave out overhealing.

These still run in Go:

- Quick Sims that ask for stat weights.
- Best Gear, trinket boards, race boards and the Build Lab.
- The one fight a Quick Sim's timeline and combat log show. It is replayed in Go, which gives
  the same fight.
- Quick Sims the coverage gate refuses, about 5% of them: some item swaps, some custom
  rotation syntax and a few spells and set bonuses. Each refusal gives a reason code.
- Quick Sims on MythicSim's second worker, which does not carry the Rust engine yet.

A character Rust preparation does not cover yet is prepared by the Go exporter, and Rust
still runs the fight.

## The Go reference

The reference pin stays at Go revision `a667dce8492bd9fb94269a9f864289cb74006775`, the fork's
patches 190 to 192 on the last Go release. Every accepted fixture keeps its Go golden, so a change to a mechanic Go covers
still has to match it or explain why it no longer should. A match means the same behavior as
that Go engine, not proof of the live game. [UPSTREAM.md](UPSTREAM.md) records how the pin
moved while Go led, and [docs/validation-summary.md](docs/validation-summary.md) lists the
comparison records.

## Rust-only mechanics

Mechanics the Go engine never had have no Go golden. They are checked against the game's own
numbers instead: the client spell rows, Wowhead Forever tooltips and logs, with focused tests
that pin the expected heal, cost or timing from those numbers. A request that reaches one runs
in Rust or is refused; it never falls back to Go, which would skip the spells it does not know.
The healer sims are the first: every Holy Paladin, Priest, Restoration Shaman and Restoration
Druid heal lands on a player who needs it, less the share real healers overheal (below), so
healing per second is what the healer's mana heals in a dungeon.

Where Rust departs from a mechanic the Go reference has, preparation follows Forever by default
and the reference with `Rules::GoReference` (`prepare --go-rules`), which every comparison with
the Go goldens and exporter uses. The departures:

- Pet hit, from Forever's developers on their tracker (22 October 2026): a Hunter's pet takes
  17/8 of its master's ranged hit as spell hit (Lightning Breath and the other magic pet hits),
  and a Warlock's demon takes 8/17 of its master's spell hit as melee hit, where the reference
  gives a Hunter pet no spell hit and a demon the whole spell hit.
- Improved Mind Flay, from its Forever tooltip ("Each instance of damage of Mind Flay deals 20%
  more damage than its previous instance of damage"): each tick deals the step more than the tick
  before it, compounding (1, 1.2 and 1.44 times the tick at two points), where the reference adds
  the step per tick (1, 1.2 and 1.4). The talent is a dummy aura run by a server-side script, so
  the client data has no stacking rule of its own.
- Level partial resists, from Forever's beta logs on foreverlogs.gg (23 October 2026): a target
  above the caster's level partially resists nothing, dot ticks and direct hits alike, so only
  resistance past the caster's Spell Piercing does. About 1,000 dot ticks and 100 direct
  non-binary hits on Razorfen Kraul and Scarlet Monastery bosses, which miss level 30 casters'
  Shadow Bolts a third of the time, all landed whole. The reference's level 63 boss partially
  resists 18% of them, 6% of the damage on average. The prepared target carries
  `no_level_partial_resist`.
- Casting regeneration auras, from the client's mana regeneration interrupt aura (type 134),
  which acts as it is applied: Resourcefulness, Rapid Recuperation and Improved Stormstrike let
  more spirit regeneration continue while casting from the moment they are up. The reference
  raises the rate without refreshing its cached mana tick, so the buff acts only once a change of
  MP5, Spirit or Intellect refreshes it, which on a Survival Hunter is almost never. Each one's
  mana lands in a resource row of its own, named after the aura, as Innervate's does. The
  prepared player carries `casting_regen_at_once`.
- Overhealing, from Forever's level 30 beta logs on foreverlogs.gg (1 to 24 October 2026, 74
  public reports): a healer sim counts each heal less the share real Holy and Discipline
  Priests, Holy Paladins and Restoration Druids and Shamans overhealed of it, such as Flash Heal
  1%, Holy Light 10%, Renew 16%, Prayer of Healing 23% and Healing Stream Totem 63%. Heals the
  logs do not hold yet (Greater Heal, Chain Heal, Prayer of Mending and the other heals above
  level 30) take the pooled share of their kind: 5.4% on one player at once, 16.7% over time,
  24.6% on a group. Shields keep their whole absorb. The shares are dungeon play at level 30 and
  will move with level 60 raids. The reference, like Go, counts every heal in full. The prepared
  player carries `overheal`, and each heal's overhealing is reported beside its healing
  (`src/prepare/overheal.rs`, `validation/2026-10-24-healer-overheal.json`).
- Heal over time crits, from Forever's level 30 beta logs on foreverlogs.gg (1 to 24 October
  2026) and the client rows: Renew, Rejuvenation, Regrowth and Healing Stream Totem ticks crit
  for 1.5 times the tick, between normal ticks on the same player (a Holy Priest's Renew ticks
  97 and crits 145 and 146, a Restoration Druid's Rejuvenation ticks 95 and crits 141 and 142).
  Pooled over 104 reports, 7.0% of 8,245 Renew ticks and 5.7% of 7,171 Rejuvenation ticks crit,
  near the casters' direct heal crit (9.4% and 5.9%); a tick that wholly overheals loses its crit
  flag, so heavy overhealers read low. Every heal over time row the healer sims use states
  Periodic Can Crit, the client bit damage ticks already follow. A tick rolls the healing crit on
  spell crit and the spell's crit bonus, as a direct heal does (Improved Regrowth's reaches
  Regrowth's ticks, as the talent names the whole spell), less what a damage tick does not take
  (Inner Focus). The reference, like Go, never crits a heal tick. Each prepared heal over time
  whose row says so carries `tick_can_crit`
  (`validation/2026-10-25-heal-tick-crits.json`).
- Conjured cooldown, from the client's item effects (1.60.1.70532, ItemEffect and
  ItemXItemEffect): every mana gem (Agate, Jade, Citrine, Ruby) is in spell category 1153 with a
  2 minute category cooldown, with Demonic and Dark Rune, the healthstones and Thistle Tea, so a
  mage who uses a gem cannot use a rune for 2 minutes, and the reverse. The reference gives the
  gems a category 30 timer of their own, which no Forever item has, so a gem and a rune went back
  to back. Forever preparation puts the gems on the category 1153 timer.
- A queued spell holds a rotation's move, from MythicSim's Discord (25 October 2026, Bae and
  MuRRRkn on the Survival weave): a player who has pressed a shot or strike in the spell queue
  window does not walk off before it fires. The reference starts the move at once, so a shot
  queued at the end of the global cooldown fired from inside its 10.5 yard minimum range and a
  Strider Kick queued as the weave stepped out fired from outside melee range, and both failed.
  On the Survival light weave that cost a Strider Kick or a shot every few cycles, and 3% haste
  lost 19 DPS because it moved the Auto Shot onto the global cooldown more often. Forever
  preparation writes `move_waits_for_queued_cast`; a spell out of range where the player stands
  does not hold the move.
- A move arrives when its time is up (`move_arrives_at_destination`, Forever preparation): a
  rotation woken at the instant a move ends, before its end action runs, reads the destination.
  The reference reads the walk rounded to the millisecond, 5.001 yards for a step in to 5, takes
  the Hunter out of melee range with no move left to bring them back, and the rest of the
  iteration has no swings or strikes: the Go golden of the Survival weave
  (`survival-hunter-aspect-of-the-beast-over-hawk`) has iterations at 400 to 510 DPS from it, and
  a held move makes it far more common.
- Water Shield gives mana (`water_shield`, Forever preparation), from the client (1.60.1.70532)
  and MythicSim's Discord (26 October 2026, Maro): a critical spell of the shaman's, damage or
  heal, spends one of three globes for 2% of maximum mana, one globe every 3.5 seconds at most,
  and the cast has no cooldown since 70532 (it had 15 seconds). The reference registers the talent
  but no build could cast it.
- Read Ley Line's Energized lasts 30 seconds (Forever preparation), as client 1.60.1.70532 has it;
  the reference keeps 70245's 15.
- A damage bonus applied after the calculation is threat too (`threat_follows_damage_bonus`,
  Forever preparation), from MythicSim's Discord (26 October 2026, Vrelk: a Shadow Priest with
  no threat talents had less threat than damage). Improved Mind Flay's ramp, Ice Lance on a
  frozen target, Soul Siphon on Drain Life and Wrack, Amplify Curse on Curse of Agony and
  Incinerate on an Immolated target raise a hit's damage after its threat is worked out; the
  reference keeps the threat of the damage before the bonus, so Mind Flay made about 0.82
  threat per damage with no threat talents.
- Party damage for heals that follow it, an assumption until the logs hold them: Forever's level
  30 beta logs have no Prayer of Mending or Chain Heal yet. A healer's party of five takes hits
  at random, the tank about once a second and each of the other four about once every five
  seconds. Prayer of Mending goes on the tank and heals only when the player holding it is hit,
  then moves to another member with 30 seconds again (its buff row, 401877), so a cast recast
  every ten seconds heals about three of its five charges. Chain Heal lands on the tank and jumps
  only to members hit in the last four seconds, 2.72 players a cast on average. Holy Priest
  healing falls about 13%, Restoration Shaman's about 0.5%. Recalibrate the rates once the logs
  reach these heals. The reference heals every charge and jump at once. The prepared `heals`
  effect carries `party`, and each such heal `smart` (`src/prepare/party_damage.rs`).
- Burst healing, a MythicSim measure rather than a game mechanic: a healer sim reports, beside
  its healing per second over the fight (sustained, which its mana spreads thin), the most it
  heals in any ten seconds of each fight over ten seconds (burst), counted as healing per second
  is: heals, ticks and shields on the healer, less overhealing. Ten seconds is a judgement: a few
  casts and a heal over time's ticks, short enough that mana does not decide it. The fixtures'
  bursts run 3 to 4.7 times their healing per second. The prepared player carries
  `burst_window`, and the report's player `burstHps` (`src/prepare/burst_hps.rs`,
  `src/core/fight/burst.rs`). The reference, like Go, reports none.
- A healer's gear, from the client rows (1.60.1.70425): the reference counts many healing items
  and set bonuses for nothing, as it registers neither them nor their sets, and Rust refused a
  healer wearing a set it registers but whose listener Rust left unclaimed. In a healer sim,
  Forever preparation (`src/prepare/healer_gear.rs`) reads the bonus rows (ItemSetSpell, through
  `tools/forever_set_spells.py`) of every set the reference does not register: healing power,
  spell damage, mana per 5 seconds, spell crit, stats, casting regeneration and spell mods, as
  Vestments of Prophecy's Flash Heal and Prayer of Healing bonuses, Vestments of Faith's Renew
  cost or the PvP sets' healing; procs and conditional bonuses are left out. It adds the healing
  bonuses the reference leaves empty: Stormrage Raiment's Regrowth cast time and Rejuvenation
  duration, The Earthfury's Mana Surge (a 25% chance after Healing Wave or Lesser Healing Wave to
  regain 35% of its base cost) and Healing Wave jumping to two more players at 20% of the heal
  before each, and The Ten Storms' 30% more Chain Heal past the first player. It wires the healing
  idols and totems (Idol of Health, Idol of Longevity, Idol of Swiftness, Totem of Life, Totem of
  Flowing Water, Tidal Totem, Totem of Urgency) and Hazza'rah's Charm of Healing and Wushoolay's
  Charm of Nature, whose use gives their rows' cast time and cost mods for 15 seconds. And it
  claims Sudden Insight (Vestments of the Devout and Magister's Regalia, 5 pieces: a cast's 5%
  chance at 200 mana) and The Furious Storm (The Elements 5, The Five Thunders 4: a cast's 4%
  chance at 65 spell power for 10 seconds), which the reference registers. Totem of Life's 80 is
  added to Lesser Healing Wave's roll, as the reference adds Libram of Light's to Flash of Light.
  More set and item bonuses that need the fight: Vestments of Faith's Armor of Faith (a Greater
  Heal crit shields its player for 500) and Epiphany (a cast's 5% chance at 24 mana per 5
  seconds for 30 seconds), Dreamwalker Raiment's Healing Touch crit returning 30% of its cost,
  Vestments of Transcendence's rank 5 Renew on each Greater Heal, and Idol of Rejuvenation's 5%
  on Regrowth's direct heal while a Rejuvenation of the druid's is up (read as "on a target
  under your Rejuvenation", since heals land on whoever needs them), Idol of Synthesis (a second
  off Swiftmend's cooldown for each heal over time up as it is cast), Garments of the Oracle (3:
  a heal's 20% chance to heal the healer for 10% of it), Rotmender's Raiment (5: a heal's cast,
  at most every 2 seconds, puts up Rotmending's 40 every 3 seconds for 15 seconds, counted at
  once), The Earthshatterer (4: the party's Mana Spring 25% stronger; 8: Lightning Shield's 15
  mana per 5 seconds, read as kept up) and Redemption Armor (4: Lay on Hands 12 minutes sooner).
  On use, Blessed Prayer Beads heal for 648 and Enamored Water Spirit's totem restores 33 mana
  every 2 seconds for 24 seconds; Wushoolay's Charm of Spirits, a Lightning Shield trinket, is
  worn without effect. Bonuses that help only the heal's target (Dreamwalker's Rejuvenation
  mana, Totemic Power, Holy Power, Totemic Power's armor) change nothing the healer heals, and
  The Eye of Divinity has no effect in Forever (its tooltip is flavour text).
  The healer trinkets on use: Talisman of Ascendance, which the reference registers, stacks
  Ascendance on each landed heal; Zandalarian Hero Charm, which it does not, starts at twelve
  stacks of 17 spell damage and 34 healing (its tooltip; the row is a dummy) and loses one each
  spell cast; Scarab Brooch shields the player each direct heal lands on for 15% of the healing
  it did, counted on the item; Natural Alignment Crystal gives heals and their ticks 20% and
  spells 20% more cost. A stacking trinket whose stacks only melee or ranged hits move, as Badge
  of the Swarmguard, is worn without effect. The prepared effects are `mana_on_cast`,
  `spell_mod_aura_on_use`, `stacking_cd_on_use`, `shield_on_heal` and the heals' shapes; a sim
  that is not a healer's keeps the reference's gear (`tests/healer_gear.rs`).

## Repository layout

| Path | Contents |
| --- | --- |
| `src/core`, `src/mechanics` | Shared fight runtime and combat rules |
| `src/classes/<class>` | Class spells and talents, with specs under `specs/` |
| `src/prepare` | Preparing a character from a request |
| `src/engine` | Coverage gate and orchestration |
| `src/contracts` | Input and output types |
| `tests/classes` | Tests, one folder per class |
| `fixtures` | Prepared inputs and Go results |
| `tools` | Exporter, routing, comparison and validation tools |

The [contributor code map](docs/contributor-guide.md) shows where each mechanic lives.

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the checks to run. A mechanics change needs a
fixture or test that shows it matches Go. Open work is on the
[contributor board](https://github.com/sage3648/mythicsim-forever-engine/issues/25).

## License and acknowledgments

MIT licensed, with new project work credited to MythicSim contributors.
Upstream wowsims notices are retained for adapted portions in [LICENSE](LICENSE).
RNG behavior, combat semantics and reference tooling derive from
[wowsims Forever](https://github.com/ElliotWood/Forever) and
[MythicSim's Go fork](https://github.com/sage3648/mythicsim-forever-engine-go).
See [NOTICE.md](NOTICE.md) for provenance. This project is not affiliated with
Blizzard Entertainment or the WoW Forever server team.
