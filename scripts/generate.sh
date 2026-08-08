#!/usr/bin/env bash
# Regenerate the Hanzo API client crate from the unified OpenAPI spec.
#
# The ONE way: hanzoai/cloud's own `openapi.yaml`, at the ref `.spec-lock` names,
# read from git.hanzo.ai. The `hanzo-client` crate is generated from it with
# openapi-generator (rust) — no Stainless, no hand-drift. Never edit
# crates/hanzo-client/src; change the handler in hanzoai/cloud and regenerate.
#
#   ./scripts/generate.sh                     # the document .spec-lock names
#   SPEC=/path/to/openapi.yaml ./scripts/generate.sh
#
# Requires: java 17+, curl, cargo, FORGE_TOKEN (contents:read on hanzoai/cloud).
set -euo pipefail
cd "$(dirname "$0")/.."

GENERATOR_VERSION="${GENERATOR_VERSION:-7.14.0}"
SPEC_REPO="${SPEC_REPO:-hanzoai/openapi}"
SPEC="${SPEC:-}"
JAR="${JAR:-${TMPDIR:-/tmp}/openapi-generator-cli-${GENERATOR_VERSION}.jar}"

check=0
[ "${1:-}" = "--check" ] && check=1

# THE DOCUMENT COMES FROM THE FORGE, AT THE REF THIS TREE NAMES. hanzoai/ci's
# client lane exports SPEC by value, already digest-checked; without it, read
# .spec-lock and fetch the same bytes ourselves. What stood here reached for
# raw.githubusercontent.com and then api.github.com, neither of which serves this
# document: the GitHub side is a mirror thousands of commits behind, so the
# "public fetch first, token fallback" ladder was two spellings of one 404 and
# the message blamed the credential. One host, one credential, one ref.
if [ -z "$SPEC" ]; then
  SPEC="$(mktemp)"
  ref="$(sed -n 's/^ref=//p' .spec-lock)"
  want="$(sed -n 's/^sha256=//p' .spec-lock)"
  : "${ref:?no .spec-lock — this tree does not name a document}"
  : "${FORGE_TOKEN:?reading ${SPEC_REPO} from git.hanzo.ai needs FORGE_TOKEN (contents:read), or pass SPEC=/path/to/the/document}"
  curl -fsSL -H "Authorization: token $FORGE_TOKEN" \
    "https://git.hanzo.ai/v1/repos/hanzoai/cloud/raw/openapi.yaml?ref=$ref" -o "$SPEC"
  got="$(sha256sum "$SPEC" | cut -d' ' -f1)"
  # A pinned ref whose bytes moved means someone moved a tag, and no amount of
  # regenerating makes that safe.
  [ "$got" = "$want" ] || { echo "hanzoai/cloud@$ref:openapi.yaml hashes to $got, but .spec-lock says $want — the ref moved under this projection" >&2; exit 1; }
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
# committed artifact: there is one document, and it is hanzoai/cloud's.
SPEC_JSON="$STAGE/hanzo.json"
python3 -c 'import json,sys,yaml; json.dump(yaml.safe_load(open(sys.argv[1])), open(sys.argv[2],"w"))' \
  "$SPEC" "$SPEC_JSON"

OUT="$STAGE/gen"
# --skip-validate-spec: the document is OpenAPI 3.1, and 3.1 made `responses`
# OPTIONAL on an operation. The validator in generator 7.14.0 still enforces the
# 3.0 rule that it is required, so it REFUSES a document that is valid. A large
# share of hanzoai/cloud's operations are routes the router proves exist and whose
# response shape no seam can state, and cloud emits those with no `responses` key
# on purpose (openapi/openapi.go — "absent stays valid and absent beats
# invented"). Without this flag the crate cannot be generated from the one
# document at all, which is exactly why the published crate was still a projection
# of the retired hand-authored master.
#
# The share is deliberately not written down here. It moves with every release,
# a number in a comment moves with nothing, and openapi/floor.json in hanzoai/cloud
# is the one place in the fleet where a count of the document is allowed to live.
#
# What keeps a bad document out is not the validator, it is `cargo build` — the
# whole generated crate plus the six example flows, in hanzo.yml's test: block.
# A malformed document fails there with a file and a line, which is strictly
# more than "Issues with the OpenAPI input" ever told anyone.
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
  --skip-validate-spec \
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
    echo "clean: crates/hanzo-client/src is what the document projects"
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
