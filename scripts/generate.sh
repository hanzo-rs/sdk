#!/usr/bin/env bash
# Regenerate the Hanzo API client crate from the unified OpenAPI spec.
#
# The ONE way: hanzoai/openapi `hanzo.yaml` is the single source of truth. The
# `hanzo-client` crate is generated from it with openapi-generator (rust) — no
# Stainless, no hand-drift. Never edit crates/hanzo-client/src; edit the
# per-service spec in hanzoai/openapi and regenerate.
#
#   ./scripts/generate.sh                            # pulls spec from hanzoai/openapi@main
#   SPEC=/path/to/hanzo.yaml ./scripts/generate.sh   # local spec override
#
# hanzoai/openapi is PRIVATE today. raw.githubusercontent.com only serves public
# repos, so the plain URL 404s; when that happens this falls back to the GitHub
# API with a token (SPEC_TOKEN, or GH_TOKEN/GITHUB_TOKEN, or `gh auth token`)
# and says so. SPEC_URL overrides the URL, SPEC overrides the file.
#
# Requires: java 17+, curl, cargo.
set -euo pipefail
cd "$(dirname "$0")/.."

GENERATOR_VERSION="${GENERATOR_VERSION:-7.14.0}"
SPEC_REPO="${SPEC_REPO:-hanzoai/openapi}"
SPEC_REF="${SPEC_REF:-main}"
SPEC_URL="${SPEC_URL:-https://raw.githubusercontent.com/${SPEC_REPO}/${SPEC_REF}/hanzo.yaml}"
SPEC="${SPEC:-}"
JAR="${JAR:-${TMPDIR:-/tmp}/openapi-generator-cli-${GENERATOR_VERSION}.jar}"

check=0
[ "${1:-}" = "--check" ] && check=1

if [ -z "$SPEC" ]; then
  SPEC="$(mktemp)"
  # Public fetch first: it is the plain path, needs no credential, and starts
  # working the day hanzoai/openapi opens. While the repo is private GitHub
  # answers 404 rather than 403, so an anonymous miss is indistinguishable from
  # a deleted file — hence the fallback below, which says which case it was.
  # Both paths use curl -f under set -e, so a failed fetch stops the script
  # instead of regenerating from a stale spec.
  if ! curl -fsSL "$SPEC_URL" -o "$SPEC"; then
    TOKEN="${SPEC_TOKEN:-${GH_TOKEN:-${GITHUB_TOKEN:-$(gh auth token 2>/dev/null || true)}}}"
    : "${TOKEN:?$SPEC_URL is not readable anonymously and no SPEC_TOKEN/GH_TOKEN is set. $SPEC_REPO is private; supply a token with contents:read, or pass SPEC=/path/to/hanzo.yaml}"
    echo "note: $SPEC_URL returned no spec ($SPEC_REPO is private) - reading it through the GitHub API instead" >&2
    curl -fsSL \
      -H "Authorization: Bearer $TOKEN" \
      -H "Accept: application/vnd.github.raw" \
      "https://api.github.com/repos/${SPEC_REPO}/contents/hanzo.yaml?ref=${SPEC_REF}" -o "$SPEC"
  fi
fi

if [ ! -f "$JAR" ]; then
  curl -fsSL -o "$JAR" \
    "https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/${GENERATOR_VERSION}/openapi-generator-cli-${GENERATOR_VERSION}.jar"
fi

STAGE="$(mktemp -d)"

# The document as JSON, because YAML has a ceiling and JSON does not.
#
# swagger-parser hands a YAML document to snakeyaml, which refuses anything over
# 3 * 1024 * 1024 = 3145728 code points. hanzo.yaml passed that mark and this
# script has been unable to generate since: measured 2026-08-01 at 3,686,318
# code points, `YAMLException: The incoming YAML document exceeds the limit:
# 3145728 code points`. The failure does not say so out loud — the parser logs
# SnakeException, falls through to the Swagger 2.0 compat reader, and dies with
# "Issues with the OpenAPI input", which reads like a malformed spec. It is not:
# the document validates at 0 errors.
#
# `-DmaxYamlCodePoints` is NOT the fix — swagger-parser honours it in generator
# 7.24.0 and ignores it in the 7.14.0 pinned here. JSON avoids snakeyaml
# altogether on every version. The generator reads either format from -i, so
# this costs one temp file and removes a ceiling the document keeps growing into.
#
# The same conversion hanzoai/openapi's generate.py and cpp-sdk's generate.sh
# already do, for the same reason, and deliberately NOT written back as a second
# committed artifact: there is one document, and it is hanzo.yaml.
SPEC_JSON="$STAGE/hanzo.json"
python3 -c 'import json,sys,yaml; json.dump(yaml.safe_load(open(sys.argv[1])), open(sys.argv[2],"w"))' \
  "$SPEC" "$SPEC_JSON"

OUT="$STAGE/gen"
# Validation stays ON. hanzo.yaml validates clean, and a malformed document
# should fail here rather than surface as a compile error spread across 1300
# generated files.
#
# --type-mappings=file=Vec<u8>: the rust generator maps a binary request body to
# std::path::PathBuf and then emits `.body(that)`, but reqwest::Body has no
# `From<PathBuf>`, so those operations would not compile. Bytes are what a
# client hands to `.body()` anyway.
#
# -t scripts/templates: one overridden template, reqwest/api.mustache, which
# emits `.body()` for a file body unconditionally - including when the body is
# optional, where the parameter is `Option<Vec<u8>>` and `reqwest::Body` has no
# `From<Option<_>>`. 14 operations here take an optional octet-stream body, so
# the override wraps that case in `if let Some(..)`. No type mapping can fix it
# because the problem is the Option, not the inner type. Templates not present
# in that directory fall back to the generator's built-ins.
java -jar "$JAR" generate \
  -i "$SPEC_JSON" -g rust \
  -t scripts/templates \
  '--type-mappings=file=Vec<u8>' \
  --additional-properties=packageName=hanzo-client,library=reqwest,supportAsync=true,supportMultipleResponses=false,preferUnsignedInt=false \
  --git-user-id=hanzo-rs --git-repo-id=sdk \
  -o "$OUT"

if [ "$check" = 1 ]; then
  # The client is generated, so the only thing that can rot is the committed
  # copy. This is what makes "never edit crates/hanzo-client/src" a fact rather
  # than a convention — and it is what lets the release train gate on this repo.
  if diff -qr "$OUT/src" crates/hanzo-client/src >/dev/null 2>&1; then
    echo "clean: crates/hanzo-client/src is what hanzo.yaml projects"
    exit 0
  fi
  echo "DRIFTED: crates/hanzo-client/src"
  # `|| true` because pipefail turns head's early exit into a SIGPIPE failure on
  # diff, which would abort the script before it could report the drift it just
  # found — a check that dies on the finding is not a check.
  { diff -qr "$OUT/src" crates/hanzo-client/src 2>&1 || true; } | head -20
  exit 1
fi

# The crate root owns Cargo.toml and README.md. Keep only the generated sources,
# replaced wholesale so a renamed or dropped operation cannot leave a stale
# module behind.
rm -rf crates/hanzo-client/src
cp -r "$OUT"/src crates/hanzo-client/src

echo "generated $(find crates/hanzo-client/src -name '*.rs' | wc -l) Rust files into crates/hanzo-client/src"
