// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {Test} from "forge-std-1.16.2/src/Test.sol";
import {LibRainDeploy} from "rain-deploy-0.1.7/src/lib/LibRainDeploy.sol";
import {LibMetaBoardDeploy} from "../../../src/lib/LibMetaBoardDeploy.sol";
import {MetaBoard} from "../../../src/concrete/MetaBoard.sol";

contract LibMetaBoardDeployTest is Test {
    function testDeployAddress() external {
        LibRainDeploy.etchZoltuFactory(vm);

        address deployedAddress = LibRainDeploy.deployZoltu(type(MetaBoard).creationCode);

        assertEq(deployedAddress, LibMetaBoardDeploy.META_BOARD_DEPLOYED_ADDRESS);
        assertTrue(address(deployedAddress).code.length > 0, "Deployed address has no code");

        assertEq(address(deployedAddress).codehash, LibMetaBoardDeploy.META_BOARD_DEPLOYED_CODEHASH);
    }

    function testExpectedCodeHash() external {
        MetaBoard metaBoard = new MetaBoard();

        assertEq(address(metaBoard).codehash, LibMetaBoardDeploy.META_BOARD_DEPLOYED_CODEHASH);
    }
}
