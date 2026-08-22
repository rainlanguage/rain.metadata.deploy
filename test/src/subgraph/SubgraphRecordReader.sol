// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {Test} from "forge-std-1.16.2/src/Test.sol";
import {LibRainDeploy} from "rain-deploy-0.1.7/src/lib/LibRainDeploy.sol";

/// Thrown when `networks.json` names a network with no declared correspondence
/// to a network this repo broadcasts to.
///
/// A revert rather than a silent skip: an unmapped network is a network whose
/// datasource claims an address on a chain this repo cannot say it deployed to,
/// which is precisely the claim these tests exist to refuse. Adding a network
/// to `networks.json` means adding its mapping here in the same change.
/// @param graphNetwork The unmapped network name, as `networks.json` spells it.
error UnmappedSubgraphNetwork(string graphNetwork);

/// One `dataSources` entry of `networks.json`, flattened: the file nests
/// datasource name under network, and every assertion here wants both keys
/// alongside the values.
struct SubgraphDataSource {
    /// The network name as The Graph spells it — the outer key.
    string graphNetwork;
    /// The datasource name — the inner key, and the name `subgraph.yaml`
    /// declares.
    string name;
    /// The address this datasource indexes.
    address deployedAddress;
    /// The block the datasource starts indexing from.
    uint256 startBlock;
}

/// @title SubgraphRecordReader
/// @notice Reads `subgraph/networks.json`: the network keys, the flattened
/// datasources, and the `LibRainDeploy` network each Graph network name
/// corresponds to. Shared between the record suite in
/// `SubgraphDeployRecord.t.sol` and the fork suite in `SubgraphStartBlock.t.sol`,
/// which must agree on what the file says while staying SEPARATE contracts: a
/// contract boundary is what `forge test --match-contract` selects at, so an
/// unreachable RPC endpoint can red the fork suite without touching an assertion
/// that reads only this repo.
abstract contract SubgraphRecordReader is Test {
    /// The subgraph's per-network deployment table, and the whole of this
    /// repo's share of the subgraph.
    string constant NETWORKS_JSON = "subgraph/networks.json";

    /// The network names, in file order.
    /// @return The outer keys of `networks.json`.
    function graphNetworks() internal view returns (string[] memory) {
        return vm.parseJsonKeys(vm.readFile(NETWORKS_JSON), "$");
    }

    /// Every datasource in `networks.json`, flattened across networks.
    ///
    /// Keys are read out of the file rather than declared here, so a network or
    /// a datasource ADDED to the file is covered by every assertion below
    /// without anyone remembering to extend a list. A check that only looked at
    /// names it already knew would be silent on exactly the hand-edit that
    /// motivates it.
    /// @return Every datasource, network-major in file order.
    function dataSources() internal view returns (SubgraphDataSource[] memory) {
        string memory json = vm.readFile(NETWORKS_JSON);
        string[] memory networks = vm.parseJsonKeys(json, "$");

        uint256 total = 0;
        for (uint256 i = 0; i < networks.length; i++) {
            total += vm.parseJsonKeys(json, string.concat("$[\"", networks[i], "\"]")).length;
        }

        SubgraphDataSource[] memory sources = new SubgraphDataSource[](total);
        uint256 offset = 0;
        for (uint256 i = 0; i < networks.length; i++) {
            string memory networkPath = string.concat("$[\"", networks[i], "\"]");
            string[] memory names = vm.parseJsonKeys(json, networkPath);
            for (uint256 j = 0; j < names.length; j++) {
                string memory sourcePath = string.concat(networkPath, "[\"", names[j], "\"]");
                sources[offset] = SubgraphDataSource({
                    graphNetwork: networks[i],
                    name: names[j],
                    deployedAddress: vm.parseJsonAddress(json, string.concat(sourcePath, ".address")),
                    startBlock: vm.parseJsonUint(json, string.concat(sourcePath, ".startBlock"))
                });
                offset++;
            }
        }
        return sources;
    }

    /// The network this repo broadcasts to, for a network name as
    /// `networks.json` spells it.
    ///
    /// Declared rather than derived because the two names disagree and there is
    /// no rule that recovers one from the other: The Graph calls Polygon
    /// "matic" and Arbitrum One "arbitrum-one", while `LibRainDeploy` — and
    /// `foundry.toml`'s `[rpc_endpoints]` with it — calls them "polygon" and
    /// "arbitrum".
    ///
    /// Only the networks actually indexed are mapped. The remaining
    /// `supportedNetworks()` entries are left out rather than guessed at: this
    /// table's job is to resolve what the file says, and inventing a Graph
    /// spelling for a chain the subgraph does not index would be a name nobody
    /// has checked, sitting in the one place that is supposed to be checking.
    /// @param graphNetwork The network name from `networks.json`.
    /// @return The matching `LibRainDeploy` network name.
    function deployNetworkFor(string memory graphNetwork) internal pure returns (string memory) {
        bytes32 key = keccak256(bytes(graphNetwork));
        if (key == keccak256(bytes("matic"))) {
            return LibRainDeploy.POLYGON;
        }
        if (key == keccak256(bytes("arbitrum-one"))) {
            return LibRainDeploy.ARBITRUM_ONE;
        }
        if (key == keccak256(bytes("base"))) {
            return LibRainDeploy.BASE;
        }
        if (key == keccak256(bytes("base-sepolia"))) {
            return LibRainDeploy.BASE_SEPOLIA;
        }
        if (key == keccak256(bytes("flare"))) {
            return LibRainDeploy.FLARE;
        }
        revert UnmappedSubgraphNetwork(graphNetwork);
    }
}
