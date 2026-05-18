#!/usr/bin/env bash
set -euo pipefail

SOFT=300
HARD=500
EXIT=0

# Build the find exclude predicates as a static argv array. Passing them as
# "${EXCLUDES_ARGS[@]}" preserves word boundaries and skips shell glob expansion
# (an earlier draft captured them into a single string variable which let the
# shell expand `.git/*` against the cwd and pass loose paths to find).
EXCLUDES_ARGS=(
  -not -path "*/target/*"
  -not -path "*/node_modules/*"
  -not -path "*/dist/*"
  -not -path "*/.sqlx/*"
  -not -path "*/.git/*"
  -not -path "./fe/src/api/schema.d.ts"
  -not -path "./fe/openapi.json"
)

# Test files: only the hard limit. Match by directory (tests/) or filename suffix.
TEST_FILES=()
while IFS= read -r line; do TEST_FILES+=("$line"); done < <(
  find . \( -path "*/tests/*" -o -name "*.test.ts" -o -name "*.e2e.ts" \) -type f "${EXCLUDES_ARGS[@]}"
)
for f in "${TEST_FILES[@]:-}"; do
  [ -z "${f:-}" ] && continue
  lines=$(wc -l < "$f")
  if (( lines > HARD )); then
    echo "ERROR: $f has $lines lines (hard limit $HARD for test files)" >&2
    EXIT=1
  fi
done

# Non-test code: soft warning + hard error.
# For Rust files, strip the trailing #[cfg(test)] mod tests { ... } block before counting.
SRC_FILES=()
while IFS= read -r line; do SRC_FILES+=("$line"); done < <(
  find . -type f \( -name "*.rs" -o -name "*.ts" -o -name "*.vue" \) \
    "${EXCLUDES_ARGS[@]}" \
    -not -path "*/tests/*" -not -name "*.test.ts" -not -name "*.e2e.ts"
)
for f in "${SRC_FILES[@]:-}"; do
  [ -z "${f:-}" ] && continue
  if [[ "$f" == *.rs ]]; then
    # Strip the test module block (line containing #[cfg(test)] to end of file).
    lines=$(awk 'BEGIN{p=1} /^#\[cfg\(test\)\]/{p=0} p{print}' "$f" | wc -l)
  else
    lines=$(wc -l < "$f")
  fi
  if (( lines > HARD )); then
    echo "ERROR: $f has $lines lines (hard limit $HARD)" >&2
    EXIT=1
  elif (( lines > SOFT )); then
    echo "WARN: $f has $lines lines (soft limit $SOFT)"
  fi
done

exit $EXIT
