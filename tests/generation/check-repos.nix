{ pkgs, cli }:

let
  reposConfig = builtins.fromJSON (builtins.readFile ./repos.json);
  repos = reposConfig.repositories;

  mkRepoCheck = repo:
    let
      goldenDir = ./golden + "/${repo.name}";
      goldenFlake = goldenDir + "/flake.nix";
      goldenAutonixDir = goldenDir + "/.autonix";
      repoSrc = pkgs.fetchFromGitHub {
        owner = repo.owner;
        repo = repo.repo;
        rev = repo.rev;
        hash = repo.hash;
      };
    in
    pkgs.runCommand "check-generate-${repo.name}" {
      nativeBuildInputs = [ cli pkgs.python3 pkgs.diffutils ];
      meta.description = "Golden test (generation) for ${repo.owner}/${repo.repo}@${repo.rev}";
    } ''
      set -euo pipefail

      echo "=== Generation Golden Test: ${repo.name} ==="
      echo "Repository: ${repo.owner}/${repo.repo}@${repo.rev}"

      WORKSPACE="$TMPDIR/workspace"
      mkdir -p "$WORKSPACE"
      cp -r ${repoSrc} "$WORKSPACE/${repo.name}"
      chmod -R u+w "$WORKSPACE/${repo.name}"

      echo "Running: autonix --detect-scope root generate ${repo.name}"
      if ! ${cli}/bin/autonix --detect-scope root generate "$WORKSPACE/${repo.name}" > "$TMPDIR/stdout.log" 2> "$TMPDIR/stderr.log"; then
        echo "CLI execution failed for ${repo.name}"
        cat "$TMPDIR/stderr.log"
        exit 1
      fi

      normalize() {
        python3 - "$1" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
raw = path.read_bytes().decode("utf-8", errors="replace")
raw = raw.replace("\r", "")

lines = raw.split("\n")
lines = [line.rstrip(" \t") for line in lines]

normalized = "\n".join(lines).rstrip("\n") + "\n"
sys.stdout.write(normalized)
PY
      }

      normalize "$WORKSPACE/${repo.name}/flake.nix" > "$TMPDIR/actual-flake-normalized.nix"
      normalize ${goldenFlake} > "$TMPDIR/expected-flake-normalized.nix"

      if ! diff -u "$TMPDIR/expected-flake-normalized.nix" "$TMPDIR/actual-flake-normalized.nix" > "$TMPDIR/flake-diff.txt"; then
        echo "flake.nix does not match golden file for ${repo.name}"
        echo ""
        echo "Expected (${goldenFlake}):"
        cat "$TMPDIR/expected-flake-normalized.nix"
        echo ""
        echo "Actual:"
        cat "$TMPDIR/actual-flake-normalized.nix"
        echo ""
        echo "Diff:"
        cat "$TMPDIR/flake-diff.txt"
        exit 1
      fi

      if [ ! -d ${goldenAutonixDir} ]; then
        echo "Missing golden .autonix directory: ${goldenAutonixDir}"
        exit 1
      fi

      if [ ! -d "$WORKSPACE/${repo.name}/.autonix" ]; then
        echo "Missing generated .autonix directory for ${repo.name}"
        exit 1
      fi

      if ! diff -ru ${goldenAutonixDir} "$WORKSPACE/${repo.name}/.autonix" > "$TMPDIR/autonix-diff.txt"; then
        echo ".autonix/ does not match golden directory for ${repo.name}"
        echo ""
        echo "Diff:"
        cat "$TMPDIR/autonix-diff.txt"
        exit 1
      fi

      mkdir -p $out
      cp "$TMPDIR/stdout.log" $out/stdout.log
      cp "$TMPDIR/stderr.log" $out/stderr.log
      cp "$WORKSPACE/${repo.name}/flake.nix" $out/flake.nix
      cp ${goldenFlake} $out/golden-flake.nix
      cp -r "$WORKSPACE/${repo.name}/.autonix" $out/actual-autonix
      cp -r ${goldenAutonixDir} $out/golden-autonix

      python3 -c 'import json,sys; print(json.dumps({"repository": "${repo.name}", "source": "${repo.owner}/${repo.repo}", "revision": "${repo.rev}", "status": "passed", "test_type": "golden-generation"}, indent=2))' > $out/summary.json

      echo "${repo.name} generation golden test passed"
    '';

in
pkgs.lib.listToAttrs (map (repo: {
  name = "generate-${repo.name}";
  value = mkRepoCheck repo;
}) repos)
