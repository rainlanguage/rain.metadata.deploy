// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {Test} from "forge-std-1.16.2/src/Test.sol";
import {IMetaV1_2} from "rain-metadata-0.1.5/src/interface/unstable/IMetaV1_2.sol";
import {LibRainDeploy} from "rain-deploy-0.1.7/src/lib/LibRainDeploy.sol";
import {DeploySuite} from "src/abstract/RainDeploySuitesBase.sol";
import {MetaBoardDeploySuites} from "src/abstract/MetaBoardDeploySuites.sol";
import {LibMetaBoardReleased} from "src/lib/LibMetaBoardReleased.sol";

/// Thrown when `networks.json` names a network with no declared correspondence
/// to a network this repo broadcasts to.
///
/// A revert rather than a silent skip: an unmapped network is a network whose
/// datasource claims an address on a chain this repo cannot say it deployed to,
/// which is precisely the claim these tests exist to refuse. Adding a network
/// to `networks.json` means adding its mapping here in the same change.
/// @param graphNetwork The unmapped network name, as `networks.json` spells it.
error UnmappedSubgraphNetwork(string graphNetwork);

/// Thrown when a suite's artifact path is not `<path>:<Name>`, which is the
/// shape the whole record declares and the shape the forge artifact layout is
/// derived from.
/// @param artifactPath The malformed path.
error MalformedArtifactPath(string artifactPath);

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

/// @title SubgraphDeployRecordTest
/// @notice The subgraph's configuration checked against this repo's deploy
/// records — the check rainlanguage/rain.metadata#134 lost and #2 moved the
/// subgraph here to get back.
///
/// Before the split, `networks.json`'s address sat in the same tree as
/// `METABOARD_DEPLOYED_ADDRESS` and could be compared to it. The split took the
/// constant away and left `networks.json` as the only place in the org naming a
/// live `MetaBoard`, with nothing to check it against. Both halves are here
/// again, so the comparison is a test rather than a convention.
///
/// This is a Solidity test, in the existing `rainix-sol` lane, deliberately.
/// The deploy record IS Solidity — `LibMetaBoardReleased` and the candidate
/// declaration are the things being compared against — so a check written
/// anywhere else would have to re-spell the record in another language and
/// could then disagree with it. It also means the check needs no docker, no
/// node and no matchstick: the lane that already gates every push runs it.
///
/// What is asserted, and what each assertion is worth TODAY:
///
/// - Released deploys are indexed (`testEveryReleasedDeployIsIndexedOnEveryNetwork`).
///   This is the assertion #2 is about, and it is EMPTY-TRUE right now: this
///   repo has cut no `sol-v*` tag, so `LibMetaBoardReleased.releasedSuites()`
///   is empty and there is nothing to demand. That is the honest state of the
///   world and it is not dressed up as more. It arms itself: the first release
///   puts an address in the record, and from that moment the subgraph must name
///   it on every network it indexes or this test is red. `mutants.toml`'s
///   released-record mutant is what shows it bites rather than passes.
///
/// - Everything else here is live today and does real work: one address per
///   datasource across networks, every indexed network a network this repo
///   broadcasts to, `subgraph.yaml` agreeing with `networks.json`, the
///   matchstick fixture agreeing with both, the indexed event being the
///   interface's event, and the ABI being this repo's own concrete artifact.
///
/// What is deliberately NOT asserted: that `networks.json`'s address is one
/// this repo has a record of. It is not — `0xfb8437Ae...` is the v1 `MetaBoard`,
/// deployed before this repo existed, and #2 rules that it stays because it is
/// what the subgraph indexes today, not a historical record to purge. Nor is
/// the candidate's address refused, because the deploy is dispatched BEFORE the
/// release is tagged, so there is a legitimate window in which `networks.json`
/// names a freshly broadcast candidate that no frozen snapshot covers yet. An
/// assertion that failed during that window would be an assertion the release
/// process has to be worked around.
contract SubgraphDeployRecordTest is MetaBoardDeploySuites, Test {
    /// The subgraph's per-network deployment table.
    string constant NETWORKS_JSON = "subgraph/networks.json";

    /// The subgraph manifest. Holds one datasource inline as the template
    /// `graph build --network` rewrites from `networks.json`.
    string constant SUBGRAPH_YAML = "subgraph/subgraph.yaml";

    /// The matchstick fixture's copy of the indexed address.
    string constant FIXTURE_ADDRESS_TS = "subgraph/tests/address.ts";

    /// The event the subgraph indexes, as a signature. Spelled here so the
    /// pair of assertions in `testSubgraphIndexesTheInterfaceEvent` can pin
    /// `subgraph.yaml`'s spelling to the interface's topic: a hash is one way,
    /// so proving the manifest declares THIS interface's event needs the
    /// preimage written down somewhere, and here is a place a mutation to
    /// either side is caught.
    string constant INDEXED_EVENT_SIGNATURE = "MetaV1_2(address,bytes32,bytes)";

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

    /// The forge artifact path a suite's `<path>:<Name>` artifact path implies,
    /// relative to the subgraph directory.
    ///
    /// Forge writes `out/<source file name>/<contract name>.json`, so the
    /// directory is the BASENAME of the declared source path and not the path
    /// itself. Derived from the record rather than written out, so that moving
    /// or renaming the concrete moves the path the manifest is held to in the
    /// same edit that moves the record.
    /// @param artifactPath The suite's `<path>:<Name>`.
    /// @return The manifest-relative path of the ABI artifact.
    function abiPathFor(string memory artifactPath) internal pure returns (string memory) {
        bytes memory path = bytes(artifactPath);

        uint256 colon = path.length;
        for (uint256 i = 0; i < path.length; i++) {
            if (path[i] == ":") {
                colon = i;
            }
        }
        if (colon == path.length || colon == 0 || colon == path.length - 1) {
            revert MalformedArtifactPath(artifactPath);
        }

        uint256 basenameStart = 0;
        for (uint256 i = 0; i < colon; i++) {
            if (path[i] == "/") {
                basenameStart = i + 1;
            }
        }

        bytes memory sourceFile = new bytes(colon - basenameStart);
        for (uint256 i = 0; i < sourceFile.length; i++) {
            sourceFile[i] = path[basenameStart + i];
        }

        bytes memory contractName = new bytes(path.length - colon - 1);
        for (uint256 i = 0; i < contractName.length; i++) {
            contractName[i] = path[colon + 1 + i];
        }

        return string.concat("../out/", string(sourceFile), "/", string(contractName), ".json");
    }

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
    function testEachDataSourceIndexesOneAddressEverywhere() external view {
        SubgraphDataSource[] memory sources = dataSources();
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

    /// `subgraph.yaml`'s inline datasource MUST match `networks.json`.
    ///
    /// The manifest carries one network's values inline as the template that
    /// `graph build --network` rewrites from `networks.json`. Nothing in the
    /// build reads the template's own numbers, so they can drift from the table
    /// indefinitely while every build stays green — and they are what a reader
    /// of the manifest sees.
    ///
    /// The template's network is discovered from the table rather than named
    /// here, and exactly one row MUST claim it: zero means the manifest points
    /// at a network the table does not cover, and the assertion would otherwise
    /// pass by checking nothing.
    function testSubgraphYamlMatchesNetworksJson() external view {
        string memory yaml = vm.readFile(SUBGRAPH_YAML);
        SubgraphDataSource[] memory sources = dataSources();

        uint256 matched = 0;
        for (uint256 i = 0; i < sources.length; i++) {
            if (!vm.contains(yaml, string.concat("network: ", sources[i].graphNetwork))) {
                continue;
            }
            matched++;
            assertTrue(
                vm.contains(yaml, string.concat("name: ", sources[i].name)),
                string.concat("subgraph.yaml does not declare datasource ", sources[i].name)
            );
            assertTrue(
                vm.contains(yaml, string.concat("address: \"", vm.toString(sources[i].deployedAddress), "\"")),
                string.concat("subgraph.yaml address is not ", vm.toString(sources[i].deployedAddress))
            );
            assertTrue(
                vm.contains(yaml, string.concat("startBlock: ", vm.toString(sources[i].startBlock))),
                string.concat("subgraph.yaml startBlock is not ", vm.toString(sources[i].startBlock))
            );
        }
        assertEq(matched, 1, "subgraph.yaml's template network is not exactly one row of networks.json");
    }

    /// The matchstick fixture MUST use an address the subgraph indexes.
    ///
    /// `tests/address.ts` is a third hand-written copy of the address, and the
    /// unit tests assert against it, so a fixture that drifts leaves the whole
    /// matchstick suite green while proving things about a contract nothing
    /// indexes.
    function testMatchstickFixtureAddressIsIndexed() external view {
        string memory fixture = vm.readFile(FIXTURE_ADDRESS_TS);
        SubgraphDataSource[] memory sources = dataSources();

        bool isIndexed = false;
        for (uint256 i = 0; i < sources.length; i++) {
            if (vm.contains(fixture, vm.toString(sources[i].deployedAddress))) {
                isIndexed = true;
                break;
            }
        }
        assertTrue(isIndexed, "tests/address.ts names no address that networks.json indexes");
    }

    /// The event the subgraph indexes MUST be the interface's event.
    ///
    /// Two assertions, because `keccak256` is one way. The manifest declares a
    /// signature STRING; the interface exposes only the topic it hashes to.
    /// Pinning the string to the manifest and the hash to the interface leaves
    /// no way for the pair to be satisfied by an event this repo's concrete
    /// does not emit: change the manifest and the first fails, change the
    /// interface and the second does.
    ///
    /// Worth having because it is the one part of the coupling the ABI does not
    /// already carry: `graph codegen` reads the artifact, so a renamed event
    /// breaks the build, but a RE-TYPED parameter still generates and still
    /// compiles, and produces a handler decoding the wrong layout.
    function testSubgraphIndexesTheInterfaceEvent() external view {
        assertTrue(
            vm.contains(vm.readFile(SUBGRAPH_YAML), string.concat("event: ", INDEXED_EVENT_SIGNATURE)),
            "subgraph.yaml does not index the expected event signature"
        );
        assertEq(
            keccak256(bytes(INDEXED_EVENT_SIGNATURE)),
            IMetaV1_2.MetaV1_2.selector,
            "the indexed signature is not the interface's MetaV1_2 topic"
        );
    }

    /// The manifest's ABI MUST be this repo's own concrete artifact.
    ///
    /// #2 rules the ABI is generated locally rather than taken from the
    /// published package, because the concrete is HERE — so what the subgraph
    /// decodes is what this repo compiles and deploys, not what a package
    /// version happens to say. The expected path is derived from the
    /// candidate's own `artifactPath`, which is the same string the explorer
    /// verification uses, so the manifest follows the concrete automatically if
    /// it is ever moved or renamed.
    function testSubgraphAbiIsThisReposConcreteArtifact() external view {
        string memory expected = abiPathFor(metaBoardCandidate().snapshot.artifactPath);
        assertTrue(
            vm.contains(vm.readFile(SUBGRAPH_YAML), string.concat("file: ", expected)),
            string.concat("subgraph.yaml does not read its ABI from ", expected)
        );
    }
}
