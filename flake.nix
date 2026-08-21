{
  description = "Flake for development workflows.";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rainix.url = "github:rainlanguage/rainix";
  };

  outputs =
    { rainix, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = rainix.pkgs.${system};
        rust-toolchain = rainix.rust-toolchain.${system};
      in
      rec {
        packages = rainix.packages.${system} // {
          # Read-only reporting over Goldsky's subgraph listing. Packaged so a
          # scheduled workflow can `nix run` it without a dev shell.
          metaboard-subgraph-report =
            (pkgs.makeRustPlatform {
              rustc = rust-toolchain;
              cargo = rust-toolchain;
            }).buildRustPackage
              {
                name = "metaboard-subgraph-report";
                src = ./.;
                doCheck = false;
                cargoLock.lockFile = ./Cargo.lock;
                buildPhase = ''
                  cargo build --release --bin metaboard-subgraph-report
                '';
                installPhase = ''
                  mkdir -p $out/bin
                  cp target/release/metaboard-subgraph-report $out/bin/
                '';
                buildInputs = with pkgs; [ openssl ];
                nativeBuildInputs =
                  with pkgs;
                  [ pkg-config ] ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.SystemConfiguration ];
              };
        };
        devShells = rainix.devShells.${system};
      }
    );

}
