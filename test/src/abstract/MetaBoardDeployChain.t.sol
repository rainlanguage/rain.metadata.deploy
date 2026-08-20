// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {RainDeployVerifyChain} from "rain-deploy-0.1.7/src/abstract/RainDeployVerifyChain.sol";
import {MetaBoardDeploySuites} from "src/abstract/MetaBoardDeploySuites.sol";

/// @title MetaBoardDeployChainTest
/// @notice Binds this repo's declaration to `RainDeployVerifyChain`: every
/// `MetaBoard` release is live, with the code it froze, on every supported
/// network.
contract MetaBoardDeployChainTest is MetaBoardDeploySuites, RainDeployVerifyChain {}
