# rain.metadata.deploy

The **deployment** half of `rain.metadata`: the concrete `MetaBoard` contract,
its deployed address + codehash pins (`LibMetaBoardDeploy`), the rolling
`src/generated/candidate/` snapshot those pins alias, the frozen per-release
snapshots under `src/generated/<tag>/`, and the deploy script.

The **library** half — the `IMetaV1_2` / `IMetaBoardV1_2` interfaces,
`LibIMetaBoardV1_2` which carries the whole of the metaboard logic, and
`LibMeta` — lives in
[`rain.metadata`](https://github.com/rainlanguage/rain.metadata) and is imported
here as the `rain-metadata` Soldeer package. The concrete `MetaBoard` is one
delegation per entry point into that library and adds no behaviour of its own.
Consumers that need only the interfaces or the libraries depend on
`rain-metadata`; consumers that need the deployed address/codehash pins depend
on `rain-metadata-deploy`.

## Releases

This is a deploy repo: releases are **manual `sol-v*` tags**, not merges.

The on-chain deploy is a separate, human-dispatched step, run **before**
tagging: the `Manual sol artifacts` workflow runs `script/Deploy.sol` for the
`metaboard` suite. Tagging then runs `rainix-tag-release`, which never
broadcasts a deploy itself; its mechanics live in rainix.

Nothing publishes on merge: a release bumps `[external.package].version` and
freezes the current `src/generated/candidate/` snapshot into a new
`src/generated/<tag>/` in lockstep.

See rainlanguage/rain.metadata#134 for the split rationale.
