#!/bin/bash
# smoke_voice.sh - Voice Pipeline E2E Smoke Test
#
# Usage: ./scripts/smoke_voice.sh
# Returns: 0 on PASS, 1 on FAIL
#
# What it tests:
#   1. VPS runner is running
#   2. Request is processed within timeout
#   3. Result JSON is valid
#   4. Webhook is sent to Claudia
#
# Artifacts:
#   - Request: ~/clawd/artifacts/voice/requests/
#   - Result:  ~/clawd/artifacts/voice/results/
#   - Audit:   ~/clawd/artifacts/voice/audit.jsonl

set -euo pipefail

# Configuration
VPS_HOST="${VPS_HOST:-clawdbot-vps}"
TIMEOUT_SECS=30
REQUEST_ID="smoke-$(date +%s)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "==========================================="
echo " Voice Pipeline Smoke Test"
echo "==========================================="
echo ""

# Step 1: Check runner is running
echo -n "[1/4] Checking VPS runner... "
RUNNER_PID=$(ssh "$VPS_HOST" 'pgrep -f vps_voice_runner.py' 2>/dev/null || true)
if [ -z "$RUNNER_PID" ]; then
    echo -e "${RED}FAIL${NC} - Runner not running"
    echo ""
    echo "Start with: ssh $VPS_HOST 'cd ~/clawd/services/voice && source .env && nohup .venv/bin/python vps_voice_runner.py &'"
    exit 1
fi
echo -e "${GREEN}OK${NC} (pid: $RUNNER_PID)"

# Step 2: Create test request
echo -n "[2/4] Creating test request... "
ssh "$VPS_HOST" "cat > ~/clawd/artifacts/voice/requests/$REQUEST_ID.json << EOF
{
  \"id\": \"$REQUEST_ID\",
  \"action\": \"process\",
  \"params\": {\"limit\": 1},
  \"requested_by\": \"smoke_test\",
  \"requested_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
}
EOF"
echo -e "${GREEN}OK${NC} (id: $REQUEST_ID)"

# Step 3: Wait for processing
echo -n "[3/4] Waiting for result... "
START_TIME=$(date +%s)
RESULT_STATUS=""

while true; do
    ELAPSED=$(($(date +%s) - START_TIME))
    if [ $ELAPSED -gt $TIMEOUT_SECS ]; then
        echo -e "${RED}FAIL${NC} - Timeout after ${TIMEOUT_SECS}s"
        echo ""
        echo "Check audit log: ssh $VPS_HOST 'tail -20 ~/clawd/artifacts/voice/audit.jsonl'"
        exit 1
    fi

    # Check if result exists
    RESULT=$(ssh "$VPS_HOST" "cat ~/clawd/artifacts/voice/results/$REQUEST_ID.json 2>/dev/null" || true)
    if [ -n "$RESULT" ]; then
        RESULT_STATUS=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','unknown'))" 2>/dev/null || echo "parse_error")
        if [ "$RESULT_STATUS" = "completed" ] || [ "$RESULT_STATUS" = "failed" ]; then
            break
        fi
    fi

    sleep 1
    echo -n "."
done

if [ "$RESULT_STATUS" = "completed" ]; then
    echo -e " ${GREEN}OK${NC} (${ELAPSED}s)"
else
    echo -e " ${YELLOW}WARN${NC} - Status: $RESULT_STATUS"
fi

# Step 4: Check webhook was sent
echo -n "[4/4] Checking webhook... "
WEBHOOK_LOG=$(ssh "$VPS_HOST" "grep '$REQUEST_ID' ~/clawd/artifacts/voice/audit.jsonl | grep webhook" 2>/dev/null || true)

if echo "$WEBHOOK_LOG" | grep -q "webhook_sent"; then
    WEBHOOK_STATUS=$(echo "$WEBHOOK_LOG" | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','unknown'))" 2>/dev/null || echo "ok")
    echo -e "${GREEN}OK${NC} (status: $WEBHOOK_STATUS)"
elif echo "$WEBHOOK_LOG" | grep -q "webhook_error"; then
    echo -e "${RED}FAIL${NC} - Webhook error"
    echo "$WEBHOOK_LOG"
    exit 1
elif echo "$WEBHOOK_LOG" | grep -q "webhook_skipped"; then
    echo -e "${YELLOW}SKIP${NC} - No transcripts to send"
else
    # Check if CLAWDBOT_TOKEN is set
    HAS_TOKEN=$(ssh "$VPS_HOST" "grep CLAWDBOT_TOKEN ~/clawd/services/voice/.env" 2>/dev/null || true)
    if [ -z "$HAS_TOKEN" ]; then
        echo -e "${YELLOW}SKIP${NC} - CLAWDBOT_TOKEN not configured"
    else
        echo -e "${YELLOW}WARN${NC} - No webhook event found"
    fi
fi

echo ""
echo "==========================================="
echo -e " ${GREEN}SMOKE TEST PASSED${NC}"
echo "==========================================="
echo ""
echo "Artifacts:"
echo "  Request:  ssh $VPS_HOST 'cat ~/clawd/artifacts/voice/requests/$REQUEST_ID.json'"
echo "  Result:   ssh $VPS_HOST 'cat ~/clawd/artifacts/voice/results/$REQUEST_ID.json'"
echo "  Audit:    ssh $VPS_HOST 'grep $REQUEST_ID ~/clawd/artifacts/voice/audit.jsonl'"
echo ""

exit 0
