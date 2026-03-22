#!/usr/bin/env bash
# =============================================================================
# AI Docs Verification (arkai)
# Enforces ownership and hygiene checks for docs/ai.
# =============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

STATE_FILE="docs/ai/SHARED-STATE.md"
THREAD_FILE=".codex-thread"

fail_count=0
warn_count=0

ok() { printf '[OK] %s\n' "$1"; }
warn() { printf '[WARN] %s\n' "$1"; warn_count=$((warn_count + 1)); }
fail() { printf '[FAIL] %s\n' "$1"; fail_count=$((fail_count + 1)); }

if [[ ! -d "docs/ai" ]]; then
  fail "Missing docs/ai directory"
fi

if [[ ! -e "$STATE_FILE" ]]; then
  fail "Missing $STATE_FILE"
fi

if [[ -L "$STATE_FILE" ]]; then
  fail "$STATE_FILE must not be a symlink"
else
  ok "$STATE_FILE is not a symlink"
fi

if [[ -f "$STATE_FILE" ]]; then
  owner_thread="$(sed -nE 's/^Owner:[[:space:]]*([A-Za-z0-9._-]+)[[:space:]]+thread.*$/\1/p' "$STATE_FILE" | head -n1)"
  if [[ -z "$owner_thread" ]]; then
    fail "$STATE_FILE must include 'Owner: <thread-name> thread ...'"
  else
    ok "Owner thread parsed: $owner_thread"
  fi
else
  owner_thread=""
fi

current_thread=""
if [[ -f "$THREAD_FILE" ]]; then
  current_thread="$(tr -d '[:space:]' < "$THREAD_FILE")"
elif [[ -n "${CODEX_THREAD:-}" ]]; then
  current_thread="$CODEX_THREAD"
fi

if [[ -z "$current_thread" ]]; then
  warn "Current thread is unknown (.codex-thread missing and CODEX_THREAD unset)"
else
  ok "Current thread: $current_thread"
fi

all_changes="$({
  git diff --name-only -- docs/ai
  git diff --cached --name-only -- docs/ai
  git ls-files --others --exclude-standard -- docs/ai
} | awk 'NF' | sort -u)"

staged_changes="$(git diff --cached --name-only | awk 'NF' | sort -u)"
staged_docs_ai="$(git diff --cached --name-only -- docs/ai | awk 'NF' | sort -u)"

shared_changed=0
if printf '%s\n' "$all_changes" | rg -qx 'docs/ai/SHARED-STATE\.md'; then
  shared_changed=1
fi

handoff_changed=0
if printf '%s\n' "$all_changes" | rg -q '^docs/ai/handoffs/.+\.md$'; then
  handoff_changed=1
fi

non_owner=0
if [[ -n "$owner_thread" && -n "$current_thread" && "$owner_thread" != "$current_thread" ]]; then
  non_owner=1
fi
if [[ -n "$owner_thread" && -z "$current_thread" ]]; then
  non_owner=1
fi

if [[ "$shared_changed" -eq 1 && "$non_owner" -eq 1 ]]; then
  fail "Non-owner thread cannot modify docs/ai/SHARED-STATE.md"
fi

if [[ "$non_owner" -eq 1 && -n "$all_changes" ]]; then
  if [[ "$handoff_changed" -eq 0 ]]; then
    fail "Non-owner updates under docs/ai require a handoff file in docs/ai/handoffs/"
  else
    ok "Non-owner change includes handoff file"
  fi
fi

# Guard isolated state-sync commits: if SHARED-STATE is staged, stage only that file.
if printf '%s\n' "$staged_docs_ai" | rg -qx 'docs/ai/SHARED-STATE\.md'; then
  staged_count="$(printf '%s\n' "$staged_changes" | awk 'NF' | wc -l | tr -d '[:space:]')"
  if [[ "$staged_count" -gt 1 ]]; then
    fail "State-sync commits must be isolated when staging SHARED-STATE.md"
  else
    ok "Staged SHARED-STATE update is isolated"
  fi
fi

# Basic handoff naming check.
while IFS= read -r f; do
  base="$(basename "$f")"
  if [[ "$base" == ".gitkeep" ]]; then
    continue
  fi
  if [[ ! "$base" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}-[a-z0-9-]+\.md$ ]]; then
    fail "handoff filename pattern mismatch: $f"
  fi
done < <(find docs/ai/handoffs -maxdepth 1 -type f | sort)

# Basic section checks for SHARED-STATE.
if [[ -f "$STATE_FILE" ]]; then
  for section in "## Scope" "## Current Snapshot" "## Active Work" "## Change Procedure"; do
    if rg -q "^${section}$" "$STATE_FILE"; then
      ok "$STATE_FILE has section: ${section#\#\# }"
    else
      fail "$STATE_FILE missing section: ${section#\#\# }"
    fi
  done
fi

printf '\nSummary: fail=%d warn=%d\n' "$fail_count" "$warn_count"
if [[ "$fail_count" -gt 0 ]]; then
  exit 1
fi
