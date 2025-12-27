{
  description = "autonix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        rustToolchainWithLLVMTools = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "llvm-tools-preview" ];
        };

        autonix = pkgs.rustPlatform.buildRustPackage {
          pname = "autonix";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = {
            description = "Automatic Nix package detection and version management";
            license = pkgs.lib.licenses.mit;
          };
        };

        unitTests = pkgs.rustPlatform.buildRustPackage {
          pname = "autonix-tests";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildPhase = ''
            echo "Running unit tests..."
            cargo test --lib --release
          '';

          installPhase = ''
            mkdir -p $out
            echo "All tests passed" > $out/test-results.txt
          '';

          doCheck = false;
          meta.description = "Unit tests for autonix version detection";
        };

        goldenTests = import ./tests/detection/check-repos.nix {
          inherit pkgs;
          cli = autonix;
        };

        coverage = pkgs.rustPlatform.buildRustPackage {
          pname = "autonix-coverage";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [
            rustToolchainWithLLVMTools
            pkgs.cargo-llvm-cov
          ];

          buildPhase = ''
            export HOME=$(mktemp -d)
            export CARGO_HOME=$HOME/.cargo

            echo "Generating coverage report..."
            cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
            echo ""
            echo "Coverage summary:"
            cargo llvm-cov report | tee coverage-summary.txt
          '';

          installPhase = ''
            mkdir -p $out
            cp lcov.info $out/
            cp coverage-summary.txt $out/

            cargo llvm-cov --all-features --workspace --html
            cp -r target/llvm-cov/html $out/

            echo "Coverage report generated successfully" > $out/coverage-results.txt
          '';

          doCheck = false;
          meta.description = "Code coverage report for autonix";
        };
      in
      {
        packages = {
          default = autonix;
          autonix = autonix;
        };

        checks = goldenTests // {
          unit-tests = unitTests;
          coverage = coverage;
        };

        apps = {
          default = {
            type = "app";
            program = "${autonix}/bin/autonix";
            meta = {
              description = "Run autonix CLI";
            };
          };
          test = {
            type = "app";
            program = toString (pkgs.writeShellScript "run-tests" ''
              set -e
              echo "Running unit tests..."
              ${pkgs.cargo}/bin/cargo test --lib
              echo ""
              echo "All unit tests passed!"
            '');
            meta = {
              description = "Run autonix unit tests";
            };
          };
          coverage = {
            type = "app";
            program = toString (pkgs.writeShellScript "run-coverage" ''
              set -e
              export PATH="${rustToolchainWithLLVMTools}/bin:${pkgs.cargo-llvm-cov}/bin:$PATH"

              echo "Generating coverage report..."
              cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
              echo ""
              echo "Generating HTML coverage report..."
              cargo llvm-cov --all-features --workspace --html
              echo ""
              echo "Coverage summary:"
              cargo llvm-cov report
              echo ""
              echo "HTML report available at: target/llvm-cov/html/index.html"
              echo "LCOV file available at: lcov.info"
            '');
            meta = {
              description = "Generate code coverage report for autonix";
            };
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain

            bacon
            cargo-watch
            cargo-edit
            cargo-llvm-cov

            jq
            diffutils
          ];

          shellHook = ''
            echo "nix-detect development environment"
            echo "Rust version: $(rustc --version)"
          '';

          RUST_BACKTRACE = "1";
        };
      }
    );
}

