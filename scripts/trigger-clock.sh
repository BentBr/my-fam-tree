#!/usr/bin/env bash
# Advance the reminder worker's clock to 06:00 Europe/Berlin on a given date
# (default: today) and run one tick — so you can exercise the reminder-digest
# pipeline locally without waiting for the real 06:00.
#
# Requires the worker built WITH the test-fixtures feature (exposes the
# clock-advance endpoint + the dinghy `clock.my-family.docker` route):
#   REMINDER_WORKER_FEATURES=test-fixtures docker compose up -d --build reminder-worker
#
# Digests are idempotent per (user, send-date): once today's digest has sent,
# re-triggering the same day is a no-op. Since this tool exists for iterating
# (change data → re-trigger → check the mail), it CLEARS that day's digest
# first so each run sends a fresh digest reflecting the current data. Set
# TRIGGER_CLOCK_KEEP=1 to keep it instead (e.g. to test the idempotency guard).
#
# Usage:
#   ./scripts/trigger-clock.sh              # 06:00 today (Europe/Berlin)
#   ./scripts/trigger-clock.sh 2026-06-08   # 06:00 on that date
#   TRIGGER_CLOCK_KEEP=1 ./scripts/trigger-clock.sh   # don't clear (idempotency test)
set -euo pipefail

URL="${WORKER_CLOCK_URL:-http://clock.my-family.docker}/__test/advance-clock"
# Resolve the target send-date (the worker's local day at 06:00 Europe/Berlin).
DATE="${1:-$(TZ=Europe/Berlin date +%F)}"

if [ -z "${TRIGGER_CLOCK_KEEP:-}" ]; then
    if docker compose exec -T postgres psql -U my_family -d my_family \
        -c "DELETE FROM reminder_digests WHERE send_date = '$DATE';" >/dev/null 2>&1; then
        echo "  cleared any existing digest for $DATE (TRIGGER_CLOCK_KEEP=1 to keep)"
    fi
fi

BODY="{\"date\":\"$DATE\"}"
echo "→ advancing worker clock to 06:00 Europe/Berlin ($DATE) via $URL"
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
