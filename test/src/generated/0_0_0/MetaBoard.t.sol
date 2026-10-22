// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {Test} from "forge-std-1.17.0/src/Test.sol";
import {LibRainDeploy} from "rain-deploy-0.1.21/src/lib/LibRainDeploy.sol";
import {DeploySuite} from "rain-deploy-0.1.21/src/abstract/RainDeploySuitesBase.sol";
import {LibMetaBoardReleased} from "src/lib/LibMetaBoardReleased.sol";

/// @title MetaBoardV1RecordTest
/// @notice The `0_0_0` release deploys the v1 board raindex `0_1_14` declares as a
/// dependency: address and runtime code hash from raindex
/// `src/generated/0_1_14/RaindexV6.sol` `DEPENDENCIES`.
contract MetaBoardV1RecordTest is Test {
    address constant RAINDEX_0_1_14_META_BOARD = address(0xfb8437AeFBB8031064E274527C5fc08e30Ac6928);
    bytes32 constant RAINDEX_0_1_14_META_BOARD_CODEHASH =
        bytes32(0x60e0735a3406074fd8f85adb2813d0d7c346337ea4bcc6f2ef4eb25077a4933c);

    function v1Suite() internal pure returns (DeploySuite memory) {
        DeploySuite[] memory suites = LibMetaBoardReleased.releasedSuites();
        for (uint256 i = 0; i < suites.length; i++) {
            if (keccak256(bytes(suites[i].suite)) == keccak256("metaboard@0_0_0")) {
                return suites[i];
            }
        }
        revert("metaboard@0_0_0 not released");
    }

    function testV1SuiteDeploysRaindexDependency() external {
        DeploySuite memory suite = v1Suite();
        LibRainDeploy.etchZoltuFactory(vm);

        address deployed = LibRainDeploy.deployZoltu(suite.creationCode);

        assertEq(deployed, RAINDEX_0_1_14_META_BOARD);
        assertEq(deployed.codehash, RAINDEX_0_1_14_META_BOARD_CODEHASH);
        assertEq(suite.storedDeployedAddress, RAINDEX_0_1_14_META_BOARD);
        assertEq(suite.storedBytecodeHash, RAINDEX_0_1_14_META_BOARD_CODEHASH);
        assertEq(suite.dependencies.length, 0);
    }
}
