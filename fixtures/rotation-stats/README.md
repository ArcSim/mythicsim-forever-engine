# Rotation line checks

Go goldens of what `sim --rotation-stats` adds to the player's metrics: `effectiveRotation`,
`rotationStats` and `rotationPresentationVersion`, as MythicSim's Go CLI writes them
(the wowsimcli of the production image at the reference pin `a667dce84`, whose
`mythicsim-forever/cli/report_rotation.go` adapter turns Go's APL validation messages into
the rotation the application shows). Each file holds those three fields of one request's
result, made with:

```sh
docker run --rm -v "$PWD":/w mythicsim-forever:a667dce84-oct22 sim --strict \
  --infile /w/NAME.request.json --outfile /w/NAME.go-result.json
```

The requests are the prepared v2 fixtures of the same name in `fixtures/mage/prepared-v2`,
at one iteration; the line checks do not depend on the iterations or the seed.
`shadow-priest-line-checks.request.json` is `apl-shadow-cooldown-reads` with four lines added:
a `spellIsKnown` of a spell the priest lacks (Go keeps the line and its "does not know spell"
message), an aura internal cooldown check of an aura the player lacks (the engine drops the
check: `ignoredCheck`), a hidden line, and an aura check on the target under the wrong tag (an
`auraHint` naming the right tag).

`tests/rotation_stats.rs` runs each request in Rust and compares. Go reports one message Rust
does not: "No unit found matching reference", which prints the reference in protobuf text
form; Rust drops the same lines without it.
