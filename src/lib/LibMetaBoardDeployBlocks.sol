// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity ^0.8.25;

import {LibRainDeploy} from "rain-deploy-0.1.35/src/lib/LibRainDeploy.sol";

/// One `MetaBoard` deployment on one network.
struct MetaBoardDeployBlock {
    /// The network, as `LibRainDeploy` names it.
    string network;
    /// The deployed `MetaBoard`.
    address deployedAddress;
    /// The first block with code at `deployedAddress` on `network`.
    uint256 blockNumber;
}

/// @title LibMetaBoardDeployBlocks
/// @notice The block each deployed `MetaBoard` first has code, per network,
/// taken once from the chain.
library LibMetaBoardDeployBlocks {
    /// Every recorded deployment block.
    /// @return The recorded deployment blocks.
    function deployBlocks() internal pure returns (MetaBoardDeployBlock[] memory) {
        MetaBoardDeployBlock[] memory blocks = new MetaBoardDeployBlock[](7);
        blocks[0] = MetaBoardDeployBlock({
            network: LibRainDeploy.POLYGON,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 92426181
        });
        blocks[1] = MetaBoardDeployBlock({
            network: LibRainDeploy.ARBITRUM_ONE,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 496974800
        });
        blocks[2] = MetaBoardDeployBlock({
            network: LibRainDeploy.BASE,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 50277465
        });
        blocks[3] = MetaBoardDeployBlock({
            network: LibRainDeploy.BASE_SEPOLIA,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 45787995
        });
        blocks[4] = MetaBoardDeployBlock({
            network: LibRainDeploy.FLARE,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 67943188
        });
        blocks[5] = MetaBoardDeployBlock({
            network: LibRainDeploy.HYPEREVM,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 43806327
        });
        blocks[6] = MetaBoardDeployBlock({
            network: LibRainDeploy.ETHEREUM,
            deployedAddress: address(0x8fD50fF9Db9835ba1B61394752A26F53D721D2a1),
            blockNumber: 25805918
        });
        return blocks;
    }
}
