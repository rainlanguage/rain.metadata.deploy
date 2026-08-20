// SPDX-License-Identifier: LicenseRef-DCL-1.0
// SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
pragma solidity =0.8.25;

import {BuildScript} from "rain-deploy-0.1.7/src/abstract/BuildScript.sol";
import {DeployCandidate} from "../src/abstract/RainDeploySuitesBase.sol";
import {MetaBoardDeploySuites} from "../src/abstract/MetaBoardDeploySuites.sol";
import {LibRainDeploySnapshot} from "rain-deploy-0.1.7/src/lib/LibRainDeploySnapshot.sol";

/// One contract's generated files: the rolling snapshot, the alias lib that
/// re-exports its pins and the released-suites lib emitted from its record.
struct GeneratedContract {
    /// Places the snapshot inside `src/generated/<dir>/` and names both
    /// generated libs.
    string contractName;
    /// Prefix for the constants the alias lib exports, e.g. `META_BOARD`.
    string constantPrefix;
    /// Snapshots are written from its `sourceCreationCode` and
    /// `snapshot.dependencies`; the released lib takes its suite key and
    /// artifact path from its `snapshot`.
    DeployCandidate candidate;
}

/// @title Build
/// @notice Generates the deploy pins for every contract this repo deploys.
/// `generatedContracts()` is the only list, read by every hook below.
///
/// `run()` (what CI regenerates against) rewrites the rolling
/// `src/generated/candidate/` snapshot, the alias lib and the released-suites
/// libs. `cutRelease()` freezes the candidate into `src/generated/<tag>/`
/// first. Frozen snapshots are append-only historical records, never
/// regenerated here.
contract Build is BuildScript, MetaBoardDeploySuites {
    /// Every contract this repo generates deploy pins for.
    /// @return The generated contracts.
    function generatedContracts() internal pure returns (GeneratedContract[] memory) {
        GeneratedContract[] memory contracts = new GeneratedContract[](1);
        contracts[0] = GeneratedContract({
            contractName: "MetaBoard", constantPrefix: "META_BOARD", candidate: metaBoardCandidate()
        });
        return contracts;
    }

    /// @inheritdoc BuildScript
    /// @dev In declaration order — the order the aggregate emits its entries
    /// in.
    function snapshotContractNames() internal pure override returns (string[] memory) {
        GeneratedContract[] memory contracts = generatedContracts();
        string[] memory names = new string[](contracts.length);
        for (uint256 i = 0; i < contracts.length; i++) {
            names[i] = contracts[i].contractName;
        }
        return names;
    }

    /// @inheritdoc BuildScript
    /// @dev Every alias lib, every released-suites lib and the aggregate over
    /// them.
    function regenerateLibs() internal override {
        GeneratedContract[] memory contracts = generatedContracts();
        for (uint256 i = 0; i < contracts.length; i++) {
            LibRainDeploySnapshot.writeAliasLib(
                vm,
                LibRainDeploySnapshot.LIB_DIR,
                contracts[i].contractName,
                contracts[i].constantPrefix,
                LibRainDeploySnapshot.CANDIDATE
            );
            LibRainDeploySnapshot.writeReleasedSuitesLib(
                vm,
                LibRainDeploySnapshot.LIB_DIR,
                recordRoot(),
                contracts[i].contractName,
                contracts[i].candidate.snapshot
            );
        }
        LibRainDeploySnapshot.writeReleasedSuitesAggregate(vm, LibRainDeploySnapshot.LIB_DIR, snapshotContractNames());
    }

    /// @inheritdoc BuildScript
    function regenerateSnapshots() internal override {
        GeneratedContract[] memory contracts = generatedContracts();
        for (uint256 i = 0; i < contracts.length; i++) {
            LibRainDeploySnapshot.writeSnapshot(
                vm,
                LibRainDeploySnapshot.CANDIDATE,
                contracts[i].contractName,
                contracts[i].candidate.sourceCreationCode,
                contracts[i].candidate.snapshot.dependencies
            );
        }
    }
}
