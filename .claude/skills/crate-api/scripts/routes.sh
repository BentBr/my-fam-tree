#!/usr/bin/env bash
# List every API route from the committed OpenAPI spec:
#   METHOD /path operationId
# The spec is generated from the code (rdt openapi), so this never drifts the
# way a hand-written route table does.
set -euo pipefail
cd "$(dirname "$0")/../../../.."
jq -r '.paths | to_entries[] | .key as $p | .value | to_entries[]
       | "\(.key | ascii_upcase) \($p) \(.value.operationId // "")"' fe/openapi.json \
    | sort -k2,2 -k1,1
