{ pkgs, cli }:

let
  reposConfig = builtins.fromJSON (builtins.readFile ./repos.json);
  repos = reposConfig.repositories;

  mkRepoCheck = repo:
    let
      goldenFile = ./golden + "/${repo.name}/flake.nix";
      repoSrc = pkgs.fetchFromGitHub {
        owner = repo.owner;
        repo = repo.repo;
        rev = repo.rev;
        hash = repo.hash;
      };
    in
    pkgs.runCommand "check-generate-${repo.name}" {
      nativeBuildInputs = [ cli pkgs.python3 ];
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
      if ! ${cli}/bin/autonix --detect-scope root generate "$WORKSPACE/${repo.name}" > "$TMPDIR/actual.nix" 2> "$TMPDIR/stderr.log"; then
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

      normalize "$TMPDIR/actual.nix" > "$TMPDIR/actual-normalized.nix"
      normalize ${goldenFile} > "$TMPDIR/expected-normalized.nix"

      if ! diff -u "$TMPDIR/expected-normalized.nix" "$TMPDIR/actual-normalized.nix" > "$TMPDIR/diff.txt"; then
        echo "Output does not match golden file for ${repo.name}"
        echo ""
        echo "Expected (${goldenFile}):"
        cat "$TMPDIR/expected-normalized.nix"
        echo ""
        echo "Actual:"
        cat "$TMPDIR/actual-normalized.nix"
        echo ""
        echo "Diff:"
        cat "$TMPDIR/diff.txt"
        exit 1
      fi

      mkdir -p $out
      cp "$TMPDIR/actual.nix" $out/output.nix
      cp "$TMPDIR/stderr.log" $out/stderr.log
      cp ${goldenFile} $out/expected.nix

      python3 -c 'import json,sys; print(json.dumps({"repository": "${repo.name}", "source": "${repo.owner}/${repo.repo}", "revision": "${repo.rev}", "status": "passed", "test_type": "golden-generation"}, indent=2))' > $out/summary.json

      echo "${repo.name} generation golden test passed"
    '';

in
pkgs.lib.listToAttrs (map (repo: {
  name = "generate-${repo.name}";
  value = mkRepoCheck repo;
}) repos)
