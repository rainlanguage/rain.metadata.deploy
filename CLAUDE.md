# CLAUDE.md

Only what a capable agent would get _wrong_ from this repo alone. Layout, dev
shells, build/test commands, dependency lists, and which command CI runs are all
discoverable and deliberately absent (rainlanguage/rainix#298).

## What this repo is

rain.metadata.deploy is the **deploy half** of `rain.metadata`: the concrete
`MetaBoard` (an `IMetaBoardV1_2` that is nothing but one delegation per entry
point into `LibIMetaBoardV1_2`) plus its deployed address + codehash pins. The
`IMeta*` **interfaces and the metaboard logic are NOT here** — they live in
`rain.metadata` and arrive as the `rain-metadata` Soldeer dependency
(`dependencies/rain-metadata-<version>/src/`). The metaboard subgraph is not
here either; it stays in `rain.metadata`, as do the metadata rust crates. The
one crate here reports on Goldsky deploys, and is not metadata logic.

## Conventions an agent would get wrong

- Pragma: concrete contracts, scripts and tests pin `=0.8.25` (exact); library
  and generated files float `^0.8.25` so downstream soldeer consumers on another
  `0.8.x` still compile them.
- Optimizer 100,000 runs; no CBOR metadata (`cbor_metadata = false`,
  `bytecode_hash = "none"`). The deployed address is a pure function of the
  bytecode (deterministic Zoltu deployer), so any of these changing moves the
  pins.
- Solidity sources need SPDX headers (LicenseRef-DCL-1.0); rust does not —
  `REUSE.toml` globs cover `crates/**/`.

## Deploy-pin invariants (the hazards)

- `src/generated/candidate/` is the **rolling** snapshot, rewritten from what
  source compiles to by `script/Build.sol` and currency-checked by CI.
  `LibMetaBoardDeploy.sol` aliases it.
- `src/generated/<tag>/` snapshots are **frozen**: `cutRelease()` freezes the
  candidate into a new tag dir; a release only ADDS one, never edits or deletes
  an existing one. CI enforces append-only.
- `[external.package].version` is the **last released** version. A normal PR
  does not bump it; only a release moves it, in lockstep with a new frozen
  `<tag>/`.
- Generated files (`src/generated/`, `src/lib/LibMetaBoardDeploy.sol`,
  `src/lib/LibMetaBoardReleased.sol`, `src/lib/LibReleasedSuites.sol`) — do not
  hand-edit; `script/Build.sol` regenerates them.

## Release / deploy shape

- The on-chain deploy is a human-dispatched `Manual sol artifacts` run
  (`workflow_dispatch`), done **before** tagging — never on merge, never part of
  the release workflow, and it is what actually broadcasts.
- A manual `sol-v<version>` tag is the sole release trigger. The release
  mechanics live in rainix's `rainix-tag-release` reusable, not here.
