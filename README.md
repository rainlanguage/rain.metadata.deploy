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

## Subgraph

`subgraph/` is the MetaBoard subgraph, moved here from `rain.metadata` by
[#2](https://github.com/rainlanguage/rain.metadata.deploy/issues/2).
`subgraph/networks.json` is a deployment record in JSON — a per-network table of
the deployed MetaBoard address and start blocks — which is the same class of
fact as `src/generated/<tag>/`, so it belongs with the deploy records rather
than with the interfaces.

Because both halves are in one tree again, the two are checked against each
other: `test/src/subgraph/SubgraphDeployRecord.t.sol` holds the manifest, the
network table and the matchstick fixture to `LibMetaBoardReleased`, to the
deploy candidate's own artifact path, and to the networks this repo broadcasts
to. It is a Solidity test in the ordinary `rainix-sol` lane, so it runs without
docker, node or matchstick.

The manifest reads its ABI from this repo's own compiled concrete
(`../out/MetaBoard.sol/MetaBoard.json`), not from the published `rain-metadata`
package.

Deploys are manual: the `Subgraph manual deploy` workflow (`workflow_dispatch`)
publishes to Goldsky under the subgraph name `metaboard`.

The Cynic GraphQL client that _consumes_ this subgraph (`crates/metaboard`,
published as `rain-metaboard-subgraph`) stays in `rain.metadata`: it is keyed by
endpoint URL and has no address or Goldsky coupling.

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
