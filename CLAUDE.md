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
(`dependencies/rain-metadata-<version>/src/`). The rust crates are not here
either: `crates/metaboard` is a Cynic client keyed by endpoint URL, with no
address or Goldsky coupling, so it stays with the library half. The metaboard
**subgraph** IS here — see below.

## Conventions an agent would get wrong

- Pragma: concrete contracts, scripts and tests pin `=0.8.25` (exact); library
  and generated files float `^0.8.25` so downstream soldeer consumers on another
  `0.8.x` still compile them.
- Optimizer 100,000 runs; no CBOR metadata (`cbor_metadata = false`,
  `bytecode_hash = "none"`). The deployed address is a pure function of the
  bytecode (deterministic Zoltu deployer), so any of these changing moves the
  pins.
- All source files need SPDX headers (LicenseRef-DCL-1.0).

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

## The subgraph (#2)

- `subgraph/networks.json` is a deploy record in JSON — the per-network table of
  the address the subgraph indexes — which is why it is here rather than in the
  library half.
- It names the **v1** `MetaBoard` (`0xfb8437Ae...`), deployed before this repo
  existed. This repo holds no record of v1 and is not meant to: that address is
  what the subgraph indexes today, NOT a historical pin to reconstruct or purge.
- `test/src/subgraph/SubgraphDeployRecord.t.sol` is the wiring the move exists
  for — manifest, table and matchstick fixture checked against
  `LibMetaBoardReleased`, the candidate's `artifactPath` and
  `LibRainDeploy.supportedNetworks()`. It is Solidity in the ordinary
  `rainix-sol` lane, so it needs no docker and no node.
- Its release-coverage assertion is EMPTY-TRUE until the first `sol-v*` tag and
  arms itself at that tag. `mutants.toml`'s `M11` is what shows it bites.
- The manifest reads its ABI from `../out/MetaBoard.sol/MetaBoard.json` — this
  repo's own concrete — so `subgraph-build` needs `forge soldeer install` first.
- The Graph and `LibRainDeploy` spell chains differently (`matic`/`polygon`,
  `arbitrum-one`/`arbitrum`). Adding a network to `networks.json` means adding
  its mapping in that test in the same change, or it fails closed.

## Release / deploy shape

- The on-chain deploy is a human-dispatched `Manual sol artifacts` run
  (`workflow_dispatch`), done **before** tagging — never on merge, never part of
  the release workflow, and it is what actually broadcasts.
- A manual `sol-v<version>` tag is the sole release trigger. The release
  mechanics live in rainix's `rainix-tag-release` reusable, not here.
