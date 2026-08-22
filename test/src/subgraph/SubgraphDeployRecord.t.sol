// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {LibRainDeploy} from "rain-deploy-0.1.7/src/lib/LibRainDeploy.sol";
import {DeploySuite} from "src/abstract/RainDeploySuitesBase.sol";
import {LibMetaBoardReleased} from "src/lib/LibMetaBoardReleased.sol";
import {SubgraphRecordReader, SubgraphDataSource} from "./SubgraphRecordReader.sol";

/// @title SubgraphDeployRecordTest
/// @notice `subgraph/networks.json` checked against this repo's deploy records
/// — the check rainlanguage/rain.metadata#134 lost and #2 moved the table here
/// to get back.
///
/// Before the split, `networks.json`'s address sat in the same tree as
/// `METABOARD_DEPLOYED_ADDRESS` and could be compared to it. The split took the
/// constant away and left `networks.json` as the only place in the org naming a
/// live `MetaBoard`, with nothing to check it against. Both halves are here
/// again, so the comparison is a test rather than a convention.
///
/// `networks.json` is ALL this repo holds under `subgraph/`
/// (rainlanguage/rain.metadata#149): the manifest, schema, mappings and
/// matchstick suite are subgraph SOURCE and stay in `rain.metadata`, which is
/// also where the manifest is pinned to the interface it indexes. So every
/// assertion below reads the network table and this repo's own records, and
/// nothing here reads a manifest — there is not one in this tree to read, and
/// one fetched at deploy time is not a thing a per-push test can hold.
///
/// This is a Solidity test, in the existing `rainix-sol` lane, deliberately.
/// The deploy record IS Solidity — `LibMetaBoardReleased` and
/// `LibRainDeploy.supportedNetworks()` are the things being compared against —
/// so a check written anywhere else would have to re-spell the record in
/// another language and could then disagree with it. It also means the check
/// needs no docker, no node and no matchstick: the lane that already gates
/// every push runs it.
///
/// What each assertion is worth TODAY:
///
/// - Released deploys are indexed (`testEveryReleasedDeployIsIndexedOnEveryNetwork`).
///   This is the assertion #2 is about, and it is LIVE: `sol-v0.1.0` froze the
///   `MetaBoard` this repo broadcast to all seven supported networks, so
///   `LibMetaBoardReleased.releasedSuites()` holds its address and the table
///   must name it on every network it indexes or this test is red. It was
///   EMPTY-TRUE from the suite's landing until that first release armed it.
///
/// - Everything else here is live too and does real work: the table parses to
///   something, one address per datasource across networks, every indexed
///   network a network this repo broadcasts to, no datasource starting at
///   genesis.
///
/// What is deliberately NOT asserted: that every address in `networks.json` is
/// one this repo has a record of. Today the table names only the released
/// `0.1.0` address, but the deploy is dispatched BEFORE the release is tagged,
/// so there is a legitimate window in which `networks.json` names a freshly
/// broadcast candidate that no frozen snapshot covers yet. An assertion that
/// failed during that window would be an assertion the release process has to
/// be worked around. (The v1 `MetaBoard` the table named before the flip is
/// gone from this repo outside git history; rainlanguage/rain.metadata.deploy#4
/// records the flip.)
contract SubgraphDeployRecordTest is SubgraphRecordReader {
    /// `networks.json` MUST describe at least one datasource.
    ///
    /// Every other assertion here loops over what this parses. A file that
    /// parsed to nothing would turn all of them green at once, which is the
    /// failure a consistency suite can least afford to have.
    function testNetworksJsonDescribesAtLeastOneDataSource() external view {
        string[] memory networks = graphNetworks();
        assertTrue(networks.length > 0, "networks.json names no networks");

        SubgraphDataSource[] memory sources = dataSources();
        assertTrue(sources.length > 0, "networks.json names no datasources");
        assertTrue(sources.length >= networks.length, "a network in networks.json declares no datasource");
    }

    /// A datasource name MUST index the same address on every network.
    ///
    /// This repo deploys through the Zoltu factory, which is CREATE2 over the
    /// creation code under a zero salt. The address is therefore a pure
    /// function of the bytes and is IDENTICAL on every chain they are
    /// broadcast to — `DeploySuite` records one address, not one per network,
    /// for exactly that reason. A per-network address is the shape of a
    /// hand-edit typo, and it is unfalsifiable by inspection because every row
    /// looks equally plausible.
    ///
    /// Per NAME rather than per file: a second deployment is added as a second
    /// datasource (`metaboard1`) across the same networks, and the file is then
    /// correctly holding two addresses.
    ///
    /// EVERYWHERE is asserted as well as SAME, because agreement between the
    /// rows that happen to exist says nothing about a row that does not. A name
    /// present on four networks and misspelled on the fifth leaves four
    /// agreeing entries and one singleton group that agrees with itself, so the
    /// address comparison alone passes on exactly the hand-edit it is for —
    /// while the fifth chain silently indexes nothing under that name.
    function testEachDataSourceIndexesOneAddressEverywhere() external view {
        SubgraphDataSource[] memory sources = dataSources();
        string[] memory networks = graphNetworks();

        for (uint256 i = 0; i < sources.length; i++) {
            for (uint256 j = i + 1; j < sources.length; j++) {
                if (keccak256(bytes(sources[i].name)) == keccak256(bytes(sources[j].name))) {
                    assertEq(
                        sources[i].deployedAddress,
                        sources[j].deployedAddress,
                        string.concat(
                            "datasource ",
                            sources[i].name,
                            " indexes a different address on ",
                            sources[i].graphNetwork,
                            " and ",
                            sources[j].graphNetwork
                        )
                    );
                }
            }

            for (uint256 n = 0; n < networks.length; n++) {
                bool present = false;
                for (uint256 j = 0; j < sources.length; j++) {
                    if (
                        keccak256(bytes(sources[j].name)) == keccak256(bytes(sources[i].name))
                            && keccak256(bytes(sources[j].graphNetwork)) == keccak256(bytes(networks[n]))
                    ) {
                        present = true;
                        break;
                    }
                }
                assertTrue(present, string.concat("datasource ", sources[i].name, " is not indexed on ", networks[n]));
            }
        }
    }

    /// Every FROZEN release MUST be indexed, on every network the subgraph
    /// indexes at all.
    ///
    /// This is the assertion the move exists for. A `sol-v*` tag freezes a
    /// snapshot of something that has ALREADY been broadcast — the deploy is
    /// dispatched first — so a release in the record is a live `MetaBoard` on
    /// every supported network, and a subgraph that does not name it is a
    /// subgraph silently missing the contract this repo deployed.
    ///
    /// On EVERY indexed network, not merely somewhere: the broadcast reaches
    /// all of them, so a release added to one network's table and forgotten on
    /// the other four is the exact hand-edit that has nothing checking it, and
    /// "indexed somewhere" would pass on it.
    ///
    /// Read from `LibMetaBoardReleased` rather than the `LibReleasedSuites`
    /// aggregate: the aggregate is every contract this repo releases, and a
    /// second contract added later would not be a `MetaBoard` the metaboard
    /// subgraph should be indexing.
    ///
    /// EMPTY-TRUE until the first release. See the contract natspec.
    function testEveryReleasedDeployIsIndexedOnEveryNetwork() external view {
        DeploySuite[] memory released = LibMetaBoardReleased.releasedSuites();
        SubgraphDataSource[] memory sources = dataSources();
        string[] memory networks = graphNetworks();

        for (uint256 i = 0; i < released.length; i++) {
            for (uint256 n = 0; n < networks.length; n++) {
                bool isIndexed = false;
                for (uint256 j = 0; j < sources.length; j++) {
                    if (
                        sources[j].deployedAddress == released[i].storedDeployedAddress
                            && keccak256(bytes(sources[j].graphNetwork)) == keccak256(bytes(networks[n]))
                    ) {
                        isIndexed = true;
                        break;
                    }
                }
                assertTrue(
                    isIndexed,
                    string.concat(
                        "released suite ",
                        released[i].suite,
                        " deployed at ",
                        vm.toString(released[i].storedDeployedAddress),
                        " is not indexed on ",
                        networks[n]
                    )
                );
            }
        }
    }

    /// Every indexed network MUST be one this repo broadcasts to.
    ///
    /// A datasource on a chain `script/Deploy.sol` never reaches is a claim
    /// this repo cannot back: whatever is at that address there, this repo did
    /// not put it there and holds no record that it is a `MetaBoard` at all.
    function testEveryIndexedNetworkIsADeployTarget() external view {
        string[] memory networks = graphNetworks();
        string[] memory supported = LibRainDeploy.supportedNetworks();

        for (uint256 i = 0; i < networks.length; i++) {
            bytes32 target = keccak256(bytes(deployNetworkFor(networks[i])));
            bool isSupported = false;
            for (uint256 j = 0; j < supported.length; j++) {
                if (keccak256(bytes(supported[j])) == target) {
                    isSupported = true;
                    break;
                }
            }
            assertTrue(
                isSupported,
                string.concat("networks.json indexes ", networks[i], ", which this repo does not deploy to")
            );
        }
    }

    /// Every datasource MUST start indexing after genesis.
    ///
    /// A `MetaBoard` cannot emit before it is deployed, so a zero start block
    /// indexes an empty range of chain at real cost. Zero is also what a
    /// missing or mistyped `startBlock` parses to, which is how it gets there.
    function testEveryDataSourceStartsAfterGenesis() external view {
        SubgraphDataSource[] memory sources = dataSources();
        for (uint256 i = 0; i < sources.length; i++) {
            assertTrue(
                sources[i].startBlock > 0,
                string.concat("datasource ", sources[i].name, " on ", sources[i].graphNetwork, " starts at genesis")
            );
        }
    }
}
