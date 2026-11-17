// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {LibRainDeploy} from "rain-deploy-0.1.33/src/lib/LibRainDeploy.sol";
import {LibMetaBoardDeployBlocks, MetaBoardDeployBlock} from "src/lib/LibMetaBoardDeployBlocks.sol";
import {SubgraphRecordReader, SubgraphDataSource} from "./SubgraphRecordReader.sol";

error StartBlockNotDeployBlock(string graphNetwork, string name, uint256 startBlock, uint256 deployBlock);

error DataSourceWithNoRecord(string graphNetwork, string name);

error RecordWithNoDataSource(string network, address deployedAddress);

/// @title SubgraphStartBlockTest
/// @notice The Graph never indexes before `startBlock`, so one past the deploy block silently drops every earlier event.
contract SubgraphStartBlockTest is SubgraphRecordReader {
    function checkStartBlocks(SubgraphDataSource[] memory sources, MetaBoardDeployBlock[] memory blocks) external pure {
        for (uint256 i = 0; i < sources.length; i++) {
            bool hasRecord = false;
            for (uint256 j = 0; j < blocks.length; j++) {
                if (isRecordFor(blocks[j], sources[i])) {
                    if (sources[i].startBlock != blocks[j].blockNumber) {
                        revert StartBlockNotDeployBlock(
                            sources[i].graphNetwork, sources[i].name, sources[i].startBlock, blocks[j].blockNumber
                        );
                    }
                    hasRecord = true;
                }
            }
            if (!hasRecord) {
                revert DataSourceWithNoRecord(sources[i].graphNetwork, sources[i].name);
            }
        }

        for (uint256 j = 0; j < blocks.length; j++) {
            bool hasDataSource = false;
            for (uint256 i = 0; i < sources.length; i++) {
                if (isRecordFor(blocks[j], sources[i])) {
                    hasDataSource = true;
                    break;
                }
            }
            if (!hasDataSource) {
                revert RecordWithNoDataSource(blocks[j].network, blocks[j].deployedAddress);
            }
        }
    }

    function isRecordFor(MetaBoardDeployBlock memory record, SubgraphDataSource memory source)
        internal
        pure
        returns (bool)
    {
        return keccak256(bytes(record.network)) == keccak256(bytes(deployNetworkFor(source.graphNetwork)))
            && record.deployedAddress == source.deployedAddress;
    }

    function baseDataSource(address deployedAddress, uint256 startBlock)
        internal
        pure
        returns (SubgraphDataSource[] memory)
    {
        SubgraphDataSource[] memory sources = new SubgraphDataSource[](1);
        sources[0] = SubgraphDataSource({
            graphNetwork: "base", name: "metaboard0", deployedAddress: deployedAddress, startBlock: startBlock
        });
        return sources;
    }

    function testEveryStartBlockIsTheRecordedDeployBlock() external view {
        this.checkStartBlocks(dataSources(), LibMetaBoardDeployBlocks.deployBlocks());
    }

    function testStartBlockOffTheRecordedDeployBlockFails(
        address deployedAddress,
        uint256 startBlock,
        uint256 deployBlock
    ) external {
        vm.assume(startBlock != deployBlock);
        SubgraphDataSource[] memory sources = baseDataSource(deployedAddress, startBlock);
        MetaBoardDeployBlock[] memory blocks = new MetaBoardDeployBlock[](1);
        blocks[0] = MetaBoardDeployBlock({
            network: LibRainDeploy.BASE, deployedAddress: deployedAddress, blockNumber: deployBlock
        });

        vm.expectRevert(
            abi.encodeWithSelector(
                StartBlockNotDeployBlock.selector, sources[0].graphNetwork, sources[0].name, startBlock, deployBlock
            )
        );
        this.checkStartBlocks(sources, blocks);
    }

    function testDataSourceWithNoRecordFails(address deployedAddress, address otherAddress, uint256 startBlock)
        external
    {
        vm.assume(deployedAddress != otherAddress);
        SubgraphDataSource[] memory sources = baseDataSource(deployedAddress, startBlock);
        MetaBoardDeployBlock[] memory blocks = new MetaBoardDeployBlock[](1);
        bytes memory noRecord =
            abi.encodeWithSelector(DataSourceWithNoRecord.selector, sources[0].graphNetwork, sources[0].name);

        blocks[0] = MetaBoardDeployBlock({
            network: LibRainDeploy.ARBITRUM_ONE, deployedAddress: deployedAddress, blockNumber: startBlock
        });
        vm.expectRevert(noRecord);
        this.checkStartBlocks(sources, blocks);

        blocks[0] =
            MetaBoardDeployBlock({network: LibRainDeploy.BASE, deployedAddress: otherAddress, blockNumber: startBlock});
        vm.expectRevert(noRecord);
        this.checkStartBlocks(sources, blocks);
    }

    function testRecordWithNoDataSourceFails(address deployedAddress, address otherAddress, uint256 blockNumber)
        external
    {
        vm.assume(deployedAddress != otherAddress);
        SubgraphDataSource[] memory sources = baseDataSource(deployedAddress, blockNumber);
        MetaBoardDeployBlock[] memory blocks = new MetaBoardDeployBlock[](2);
        blocks[0] = MetaBoardDeployBlock({
            network: LibRainDeploy.BASE, deployedAddress: deployedAddress, blockNumber: blockNumber
        });

        blocks[1] = MetaBoardDeployBlock({
            network: LibRainDeploy.ARBITRUM_ONE, deployedAddress: deployedAddress, blockNumber: blockNumber
        });
        vm.expectRevert(
            abi.encodeWithSelector(RecordWithNoDataSource.selector, LibRainDeploy.ARBITRUM_ONE, deployedAddress)
        );
        this.checkStartBlocks(sources, blocks);

        blocks[1] = MetaBoardDeployBlock({
            network: LibRainDeploy.BASE, deployedAddress: otherAddress, blockNumber: blockNumber
        });
        vm.expectRevert(abi.encodeWithSelector(RecordWithNoDataSource.selector, LibRainDeploy.BASE, otherAddress));
        this.checkStartBlocks(sources, blocks);
    }
}
