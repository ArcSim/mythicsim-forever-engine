# Forever gap requests

Production Quick Sim requests that the Rust engine refused on 24 to 26 October 2026, which then
ran in the Go engine. The character names are replaced and the iterations cut to 50. Each one
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
