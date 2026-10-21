# MythicSim Forever Engine

A Rust simulation engine for WoW Forever. [MythicSim](https://mythicsim.com/wow-forever/sim)
runs Forever Quick Sims on it, and the Go engine for the rest; [Coverage](#coverage) lists
which is which.

The engine follows [MythicSim's Go fork](https://github.com/sage3648/mythicsim-forever-engine-go)
of [wowsims Forever](https://github.com/ElliotWood/Forever) exactly: event order, random
numbers and float math match, so a sim gives the same result and the same combat log in
either engine. Rust is 1.3x to 2x faster.

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
their talents, gear, buffs, consumables, pets, tanking and 2 to 5 targets.

These still run in Go:

- Quick Sims that ask for stat weights.
- Best Gear, trinket boards, race boards and the Build Lab.
- The one fight a Quick Sim's timeline and combat log show. It is replayed in Go, which gives
  the same fight.
- Quick Sims the coverage gate refuses, about 5% of them: some item swaps, some custom
  rotation syntax and a few spells and set bonuses. Each refusal gives a reason code.
- Quick Sims on MythicSim's second worker, which does not carry the Rust engine yet.

A character Rust preparation does not cover yet is prepared by the Go exporter, and Rust
still runs the fight. Go also stays the reference every Rust change is compared against.

## Keeping up with Go

Each Go engine release moves the reference pin, currently Go revision
`62b7445d1c812b257110e51a5b1e8db93b987972`, and Rust follows the changes. A match means the
same behavior as that Go engine, not proof of the live game. [UPSTREAM.md](UPSTREAM.md) has
the steps, and [docs/validation-summary.md](docs/validation-summary.md) lists the comparison
records.

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
