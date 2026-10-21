#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-DCL-1.0
# SPDX-FileCopyrightText: Copyright (c) 2020 Rain Open Source Software Ltd
#
# Installs what `forge script ./script/Build.sol` staged.
#
# `BuildScript.run()` generates `foundry.toml`'s network sections and
# `.env.example`'s endpoint variables from the declared networks' catalogue entries
# but CANNOT write the first of them: foundry refuses every filesystem cheatcode
# write to the project root's own `foundry.toml`, whatever `fs_permissions`
# says. So the script splices and writes to `.staged-config/`, and this moves
# each staged file onto the file of the same name at the root.
#
# This is `rainix-copy-artifacts`' own consumer hook. rainix runs it outside any
# devshell, after `forge script ./script/Build.sol` and before the `git diff`
# that fails a stale tree — so a tree whose config has drifted from the roster
# it pins is red there. Nothing here needs forge, nix or `--ffi`; granting
# `--ffi` to the build is the alternative, and it would be granted to every
# consumer's build rather than to this one step.
set -euo pipefail

cd "$(dirname "$0")/.."

staged=.staged-config

# Absent or empty means the staging never happened, which is exactly the state
# in which copying nothing would leave the committed config stale and report
# success. The regeneration step runs before this one and fails on its own, so
# reaching here with nothing staged is a build that generated nothing.
if [ ! -d "$staged" ]; then
  echo "::error::$staged/ does not exist. forge script ./script/Build.sol stages the generated config there; it did not run, or it wrote nothing." >&2
  exit 1
fi

installed=0
while IFS= read -r -d '' file; do
  cp "$file" "./$(basename "$file")"
  installed=$((installed + 1))
done < <(find "$staged" -mindepth 1 -maxdepth 1 -type f -print0)

if [ "$installed" -eq 0 ]; then
  echo "::error::$staged/ holds no files. forge script ./script/Build.sol stages the generated config there; it wrote nothing." >&2
  exit 1
fi

# Removed once installed, so the staging directory is never a place a stale
# generated file can sit: the next run stages fresh, and a bare re-run of this
# hook fails above rather than re-installing what a previous run left.
rm -rf "$staged"

echo "Installed $installed generated config file(s) from $staged/."
