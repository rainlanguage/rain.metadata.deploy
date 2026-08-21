#!/usr/bin/env sh
# Suite entry point for mutation-probe. Kept as a script so the probe runs
# exactly one argv and the tally it reads always comes from a full run.
set -eu
exec nix develop .#rust-shell -c cargo test --workspace
