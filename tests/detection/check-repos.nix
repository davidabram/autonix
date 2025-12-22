{ pkgs, cli }:

let
  reposConfig = builtins.fromJSON (builtins.readFile ./repos.json);
  repos = reposConfig.repositories;

  mkRepoCheck = repo:
    let
      goldenFile = ./golden + "/${repo.name}.json";
      repoSrc = pkgs.fetchFromGitHub {
        owner = repo.owner;
        repo = repo.repo;
        rev = repo.rev;
        hash = repo.hash;
      };
    in
    pkgs.runCommand "check-${repo.name}" {
      nativeBuildInputs = [ cli pkgs.jq ];
      meta.description = "Golden test for ${repo.owner}/${repo.repo}@${repo.rev}";
    } ''
      set -euo pipefail

      echo "=== Golden Test: ${repo.name} ==="
      echo "Repository: ${repo.owner}/${repo.repo}@${repo.rev}"

      WORKSPACE="$TMPDIR/workspace"
      mkdir -p "$WORKSPACE"
      cp -r ${repoSrc} "$WORKSPACE/${repo.name}"
      chmod -R u+w "$WORKSPACE/${repo.name}"

      cd "$WORKSPACE/${repo.name}"
      echo "Running: autonix --format json"
      if ! ${cli}/bin/autonix --format json > "$TMPDIR/actual.json" 2> "$TMPDIR/stderr.log"; then
        echo "CLI execution failed for ${repo.name}"
        cat "$TMPDIR/stderr.log"
        exit 1
      fi

      if ! jq empty "$TMPDIR/actual.json" 2>/dev/null; then
        echo "Invalid JSON output from CLI"
        cat "$TMPDIR/actual.json"
        exit 1
      fi

      jq --sort-keys '
        .languages |= sort_by(.language) |
        .versions |= sort_by(.language)
      ' "$TMPDIR/actual.json" > "$TMPDIR/actual-normalized.json"
      jq --sort-keys '
        .languages |= sort_by(.language) |
        .versions |= sort_by(.language)
      ' ${goldenFile} > "$TMPDIR/expected-normalized.json"

      if ! diff -u "$TMPDIR/expected-normalized.json" "$TMPDIR/actual-normalized.json" > "$TMPDIR/diff.txt"; then
        echo "Output does not match golden file for ${repo.name}"
        echo ""
        echo "Expected (${goldenFile}):"
        cat "$TMPDIR/expected-normalized.json"
        echo ""
        echo "Actual:"
        cat "$TMPDIR/actual-normalized.json"
        echo ""
        echo "Diff:"
        cat "$TMPDIR/diff.txt"
        exit 1
      fi

      mkdir -p $out
      cp "$TMPDIR/actual.json" $out/output.json
      cp "$TMPDIR/stderr.log" $out/stderr.log
      cp ${goldenFile} $out/expected.json

      jq -n \
        --arg name "${repo.name}" \
        --arg repo "${repo.owner}/${repo.repo}" \
        --arg rev "${repo.rev}" \
        --arg status "passed" \
        '{
          repository: $name,
          source: $repo,
          revision: $rev,
          status: $status,
          test_type: "golden"
        }' > $out/summary.json

      echo "${repo.name} golden test passed"
    '';

in
pkgs.lib.listToAttrs (map (repo: {
  name = repo.name;
  value = mkRepoCheck repo;
}) repos)
