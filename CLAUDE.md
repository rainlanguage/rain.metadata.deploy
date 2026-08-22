# CLAUDE.md

Only what a capable agent would get _wrong_ from this repo alone. Layout, dev
shells, build/test commands, dependency lists, and which command CI runs are all
discoverable and deliberately absent (rainlanguage/rainix#298).

## What this repo is

rain.metadata.deploy is the **deploy half** of `rain.metadata`: the concrete
`MetaBoard` (an `IMetaBoardV1_2` that is nothing but one delegation per entry
point into `LibIMetaBoardV1_2`) plus its deployed address + codehash pins. The
`IMeta*` **interfaces and the metaboard logic are NOT here** — they live in
`rain.metadata` and arrive as the `rain-metadata` Soldeer dependency. The
subgraph SOURCE and the metadata rust crates stay in `rain.metadata` too Here:
the subgraph's deployment record — see below — and one crate reporting on
Goldsky deploys, which is not metadata logic.

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

## The subgraph: one file here (#2, recut by rain.metadata#149)

- `subgraph/networks.json` (per-network address + start block) is a deploy
  record and the WHOLE of this repo's share. Manifest, schema, mappings and
  matchstick suite are SOURCE and stay in `rain.metadata`, which pins the
  manifest to the interface it indexes. `Subgraph manual deploy` fetches that
  source (`metadata-ref`) and merges it in beside the table, and `graph build`
  rewrites the manifest in place — hence `.gitignore` ignores all of `subgraph/`
  except the table. Nothing else here runs a subgraph command.
- The table names the **v1** `MetaBoard` (`0xfb8437Ae...`), deployed before this
  repo existed. This repo pins no v1 bytecode and is not meant to: that address
  is what the subgraph indexes today, NOT a historical pin to reconstruct or
  purge.
- `SubgraphDeployRecord.t.sol`'s release-coverage assertion is EMPTY-TRUE until
  the first `sol-v*` tag and arms itself there. A mutation that puts a release
  into the record shows it bites.
- The Graph and `LibRainDeploy` spell chains differently (`matic`/`polygon`,
  `arbitrum-one`/`arbitrum`). Adding a network to `networks.json` means adding
  its mapping in that test in the same change, or it fails closed.
- The Goldsky version is `<address>-<short commit of THIS repo>`, not of the
  source, so two dispatches from one commit against different `metadata-ref`s
  collide and the second is skipped as already deployed (rainix#354).

## Release / deploy shape

- The on-chain deploy is a human-dispatched `Manual sol artifacts` run
  (`workflow_dispatch`), done **before** tagging — never on merge, never part of
  the release workflow, and it is what actually broadcasts.
- A manual `sol-v<version>` tag is the sole release trigger. The release
  mechanics live in rainix's `rainix-tag-release` reusable, not here.
