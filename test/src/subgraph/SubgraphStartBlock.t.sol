// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {SubgraphRecordReader, SubgraphDataSource} from "./SubgraphRecordReader.sol";

/// @title SubgraphStartBlockTest
/// @notice `startBlock` checked against the chain itself: for every datasource
/// in `subgraph/networks.json`, `eth_getCode` at `startBlock` finds code and at
/// `startBlock - 1` finds none. Together the two reads pin `startBlock` as THE
/// deployment block — the one field of the table `SubgraphDeployRecordTest`
/// cannot see past, because that suite compares the file to this repo's
/// records and no record here says when a released `MetaBoard` reached each chain.
/// All it can demand of `startBlock` is that it is not genesis.
///
/// The error that matters is the too-late direction. The Graph indexes forward
/// from `startBlock` and never revisits earlier blocks, so a `startBlock` past
/// the deploy block drops every event in the gap from the subgraph forever,
/// and no query shows the absence as anything but "there was no event". The
/// too-early direction merely indexes empty chain at real cost. Pinning the
/// exact block refuses both.
///
/// Two forks per datasource, on the datasource's own network, resolved through
/// the same declared mapping the record suite uses. The historical reads need
/// archive state — a start block only recedes as the chain grows — which is CI's
/// rpc-preflight's job: it binds each `[rpc_endpoints]` alias to an endpoint
/// that answered archive probes at or below the org's deepest pins for that
/// network, and every block this table names is at or above those probes.
///
/// A separate contract from the record suite for the same reason
/// `RainDeployVerifyChain` is separate from the snapshot checks: a contract
/// boundary is what `forge test --match-contract` and a CI job select at, so
/// an RPC outage reds this suite alone and legibly — a fork that cannot be
/// created is an outage, while an assertion failing on a fork that was created
/// is a wrong `startBlock`. Nothing reachable from the record contract forks
/// anything.
contract SubgraphStartBlockTest is SubgraphRecordReader {
    /// Every datasource's `startBlock` MUST be its address's deployment block:
    /// code there at `startBlock`, no code one block earlier.
    ///
    /// The two reads bite one direction each:
    ///
    /// - `startBlock` too LATE (deploy block + n): the code was already there
    ///   a block earlier, so the empty-before read fails. This is the
    ///   silent-gap direction.
    /// - `startBlock` too EARLY (deploy block - n): no code at `startBlock`
    ///   yet, so the code-at-start read fails first.
    ///
    /// `startBlock - 1` cannot underflow into a bogus fork: the record suite
    /// refuses a genesis start, and here a zero `startBlock` panics the
    /// subtraction — an arithmetic red rather than a semantic one, but red.
    function testEveryStartBlockIsTheDeployBlock() external {
        SubgraphDataSource[] memory sources = dataSources();
        for (uint256 i = 0; i < sources.length; i++) {
            string memory network = deployNetworkFor(sources[i].graphNetwork);

            // createSelectFork returns a fork id that is not needed here; bind
            // and reference it so the unused-return lint stays satisfied.
            uint256 forkId = vm.createSelectFork(network, sources[i].startBlock);
            (forkId);
            assertTrue(
                sources[i].deployedAddress.code.length > 0,
                string.concat(
                    "datasource ",
                    sources[i].name,
                    " on ",
                    sources[i].graphNetwork,
                    " has no code at startBlock ",
                    vm.toString(sources[i].startBlock),
                    "; startBlock is before the deploy block"
                )
            );

            forkId = vm.createSelectFork(network, sources[i].startBlock - 1);
            (forkId);
            assertEq(
                sources[i].deployedAddress.code.length,
                0,
                string.concat(
                    "datasource ",
                    sources[i].name,
                    " on ",
                    sources[i].graphNetwork,
                    " already has code one block before startBlock ",
                    vm.toString(sources[i].startBlock),
                    "; startBlock is past the deploy block and every earlier event is silently dropped"
                )
            );
        }
    }
}
