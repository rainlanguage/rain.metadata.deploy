// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity ^0.8.25;

import {
    DeployCandidate,
    DeployDeclaration,
    DeployDependency,
    DeploySuite,
    RainDeploySuitesBase
} from "rain-deploy-0.1.33/src/abstract/RainDeploySuitesBase.sol";
import {
    CREATION_CODE as META_BOARD_CREATION_CODE_CANDIDATE,
    RUNTIME_CODE as META_BOARD_RUNTIME_CODE_CANDIDATE
} from "../generated/candidate/MetaBoard.sol";
import {LibRainDeployDeclaration} from "rain-deploy-0.1.33/src/lib/LibRainDeployDeclaration.sol";
import {LibRainDeployZoltuMechanism} from "rain-deploy-0.1.33/src/lib/LibRainDeployZoltuMechanism.sol";
import {LibMetaBoardDeploy} from "../lib/LibMetaBoardDeploy.sol";
import {LibReleasedSuites} from "../lib/LibReleasedSuites.sol";

/// @title MetaBoardDeploySuites
/// @notice Everything this repo deploys, declared ONCE: the hand-written
/// `metaboard` candidate below, and the released side read from the generated
/// `LibReleasedSuites`, which `script/Build.sol` emits from the frozen record.
///
/// It lives in `src/` rather than `test/` because `.soldeerignore` excludes
/// `test/` from the published package, and in a deploy repo the deployment
/// process is the product.
abstract contract MetaBoardDeploySuites is RainDeploySuitesBase {
    /// @inheritdoc RainDeploySuitesBase
    function declaration() internal pure override returns (DeployDeclaration memory) {
        DeployDeclaration memory declared;
        LibRainDeployDeclaration.extend(declared);
        LibRainDeployZoltuMechanism.extend(declared);
        LibRainDeployDeclaration.addReleased(declared, LibReleasedSuites.releasedSuites());
        DeployCandidate[] memory candidates = new DeployCandidate[](1);
        candidates[0] = metaBoardCandidate();
        LibRainDeployDeclaration.addCandidates(declared, candidates);
        return declared;
    }

    /// This repo's rolling `MetaBoard` candidate. Named rather than reached by
    /// index into `declaration().candidates`, because `script/Build.sol` emits the
    /// released-suites lib from THIS candidate specifically, and naming it
    /// keeps the suite key, the artifact path and the dependency list spelled
    /// once.
    ///
    /// `MetaBoard` has no constructor and its one entry point reads nothing off
    /// chain, so it has no dependency that must already be deployed.
    /// @return The candidate.
    function metaBoardCandidate() internal pure returns (DeployCandidate memory) {
        return DeployCandidate({
            snapshot: DeploySuite({
                suite: "metaboard",
                creationCode: META_BOARD_CREATION_CODE_CANDIDATE,
                storedDeployedAddress: LibMetaBoardDeploy.META_BOARD_DEPLOYED_ADDRESS,
                storedBytecodeHash: LibMetaBoardDeploy.META_BOARD_DEPLOYED_CODEHASH,
                storedRuntimeCode: META_BOARD_RUNTIME_CODE_CANDIDATE,
                artifactPath: "src/concrete/MetaBoard.sol:MetaBoard",
                dependencies: new DeployDependency[](0),
                mechanismData: LibRainDeployZoltuMechanism.mechanismData(),
                networks: new string[](0),
                describedByMeta: ""
            }),
            unanchorableReason: ""
        });
    }
}
