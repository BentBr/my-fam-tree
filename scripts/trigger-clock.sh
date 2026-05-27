#!/usr/bin/env bash
# Advance the reminder worker's clock to 06:00 Europe/Berlin on a given date
# (default: today) and run one tick — so you can exercise the reminder-digest
# pipeline locally without waiting for the real 06:00.
#
# Requires the worker built WITH the test-fixtures feature (exposes the
# clock-advance endpoint + the dinghy `clock.my-family.docker` route):
#   REMINDER_WORKER_FEATURES=test-fixtures docker compose up -d --build reminder-worker
#
# Usage:
#   ./scripts/trigger-clock.sh              # 06:00 today (Europe/Berlin)
#   ./scripts/trigger-clock.sh 2026-06-08   # 06:00 on that date
set -euo pipefail

URL="${WORKER_CLOCK_URL:-http://clock.my-family.docker}/__test/advance-clock"
DATE="${1:-}"
if [ -n "$DATE" ]; then
    BODY="{\"date\":\"$DATE\"}"
else
    BODY="{}"
fi

echo "→ advancing worker clock to 06:00 Europe/Berlin (${DATE:-today}) via $URL"
if ! curl -fsS -X POST "$URL" -H 'content-type: application/json' -d "$BODY"; then
    echo
    echo "✗ could not reach the clock endpoint. Is the worker built with test-fixtures?"
    echo "  REMINDER_WORKER_FEATURES=test-fixtures docker compose up -d --build reminder-worker"
    exit 1
fi

echo
echo "✓ tick ran. A digest only sends if a reminders-enabled user has a person"
echo "  whose birthday is lead_days ahead of the trigger date. Check Mailpit:"
echo "  http://mail.my-family.docker"
