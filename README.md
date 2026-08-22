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

`subgraph/networks.json` is a deployment record in JSON — a per-network table of
the deployed MetaBoard address and start blocks — which is the same class of
fact as `src/generated/<tag>/`, so it belongs with the deploy records rather
than with the interfaces
([#2](https://github.com/rainlanguage/rain.metadata.deploy/issues/2)).

It is the only file this repo holds under `subgraph/`. The manifest, schema,
mappings and matchstick suite are subgraph SOURCE and stay in `rain.metadata`
([rain.metadata#149](https://github.com/rainlanguage/rain.metadata/issues/149)),
whose `subgraph.yaml` is a template carrying no address, start block or real
network name. `graph build --network <x>` fills all three from the table beside
it.

Because the table and the deploy records are in one tree, they are checked
against each other: `test/src/subgraph/SubgraphDeployRecord.t.sol` holds the
network table to `LibMetaBoardReleased` and to the networks this repo broadcasts
to. It is a Solidity test in the ordinary `rainix-sol` lane, so it runs without
docker, node or matchstick.

Deploys are manual. The `Subgraph manual deploy` workflow (`workflow_dispatch`,
with a `metadata-ref` input naming the subgraph source revision) checks out that
source, merges it in beside `networks.json`, builds the ABI the manifest reads,
and publishes to Goldsky under the subgraph name `metaboard`.

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

## Deployed subgraph reporting

`crates/metaboard-subgraph-report` enumerates the subgraphs deployed on Goldsky
and reports which deployed versions have been superseded, so unused deploys can
be found and retired. Run it with a Goldsky token in `GOLDSKY_TOKEN`:

```bash
GOLDSKY_TOKEN=… nix run .#metaboard-subgraph-report
GOLDSKY_TOKEN=… nix run .#metaboard-subgraph-report -- --format json
GOLDSKY_TOKEN=… nix run .#metaboard-subgraph-report -- --format candidates
```

**It never deletes anything.** The client can issue exactly one request — the
listing `GET` — and there is no delete, pause or mutate path in the crate at
all. The output is `name/version` identifiers plus the reason each was selected.
Reaping stays a human running `goldsky subgraph delete`.

### Supersession, not usage

Goldsky's subgraph admin API exposes no per-subgraph usage metrics: no query
count, no bandwidth, no last-query timestamp. "Nothing queries this" therefore
cannot be established from the API, and this tool never claims it.

What it reports instead is supersession, which is the residue the deploy leaves
behind: `subgraph-deploy` is idempotent **by name and version**, so it skips a
version already deployed and never removes the one it replaced. Every old
`<address>-<commit>` slot stays live indefinitely. A deployed version is
reported as a reaping candidate when it is all of:

- not the target of any Goldsky tag on its name,
- not the newest version for its name,
- not `--keep`-pinned by the caller, and
- at least `--min-age-days` old (default 30).

Anything that cannot be positively established is retained, with its reason
recorded: a false retention costs nothing, a false candidate risks a live
subgraph. Confirm nothing queries a candidate before reaping it.

The default `--name-prefix` is `metaboard`, matching the current deploy rule's
`metaboard-<network>` names. Older live deploys use other stems, so
`--name-prefix` is repeatable and `--name-prefix ""` sweeps everything.
