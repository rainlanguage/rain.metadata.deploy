// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {RainDeployVerifySnapshot} from "rain-deploy-0.1.7/src/abstract/RainDeployVerifySnapshot.sol";
import {MetaBoardDeploySuites} from "src/abstract/MetaBoardDeploySuites.sol";

/// @title MetaBoardDeploySnapshotTest
/// @notice Binds this repo's declaration to `RainDeployVerifySnapshot`: every
/// deploy-pin assertion over the `MetaBoard` suites that needs no network.
contract MetaBoardDeploySnapshotTest is MetaBoardDeploySuites, RainDeployVerifySnapshot {}
