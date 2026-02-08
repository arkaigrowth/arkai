#!/usr/bin/env python3
"""
Validate queue task JSON files against the queue_task_contract schema.

Supports both request files (agent → host) and state files (host → agent).
Auto-detects file type based on content, or use --request / --state flags.

Usage:
    # Validate request files
    python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_request_valid.json

    # Validate state files
    python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_state_done.json

    # Force file type (skip auto-detection)
    python3 contracts/validate_queue_task.py --request contracts/fixtures/queue_task_request_valid.json
    python3 contracts/validate_queue_task.py --state contracts/fixtures/queue_task_state_done.json

    # Check expired lease recovery
    python3 contracts/validate_queue_task.py --check-lease contracts/fixtures/queue_task_state_expired_lease.json

Exit codes:
    0 — All files valid
    1 — One or more files failed validation
    2 — Usage error
"""

import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# ── Request schema constants ──────────────────────────────────────────

TASK_ID_PATTERN = re.compile(r"^[a-f0-9]{12}$")

VALID_SERVICES = {"notes", "reminders", "imessage"}
VALID_OPERATIONS = {"list", "read", "create", "search", "send", "delete"}
VALID_PRIORITIES = {"normal", "high"}

REQUIRED_REQUEST_FIELDS = {"schema_version", "task_id", "created_at", "action"}
ALLOWED_REQUEST_FIELDS = {
    "schema_version", "task_id", "created_at", "action",
    "priority", "idempotency_key", "max_attempts", "visibility_timeout_sec",
}

REQUIRED_ACTION_FIELDS = {"service", "operation"}
ALLOWED_ACTION_FIELDS = {"service", "operation", "params"}

# ── State schema constants ────────────────────────────────────────────

VALID_STATUSES = {"pending", "leased", "done", "failed", "dead"}
TERMINAL_STATUSES = {"done", "dead"}

REQUIRED_STATE_FIELDS = {"schema_version", "task_id", "status", "attempts"}
ALLOWED_STATE_FIELDS = {
    "schema_version", "task_id", "status", "attempts",
    "lease_owner", "lease_until", "first_seen_at",
    "last_attempt_at", "completed_at", "dead_at",
    "result", "error",
}

# ── Error schema constants ────────────────────────────────────────────

VALID_ERROR_TYPES = {"transient", "permanent"}
VALID_ERROR_CODES = {
    "TIMEOUT", "RATE_LIMITED", "CLI_ERROR", "SERVICE_UNAVAILABLE",
    "INVALID_PARAMS", "PERMISSION_DENIED", "NOT_FOUND", "UNSUPPORTED_OPERATION",
}
TRANSIENT_CODES = {"TIMEOUT", "RATE_LIMITED", "CLI_ERROR", "SERVICE_UNAVAILABLE"}
PERMANENT_CODES = {"INVALID_PARAMS", "PERMISSION_DENIED", "NOT_FOUND", "UNSUPPORTED_OPERATION"}

REQUIRED_ERROR_FIELDS = {"type", "code", "message"}
REQUIRED_RESULT_FIELDS = {"service", "operation", "data"}


# ── Detection ─────────────────────────────────────────────────────────


def detect_file_type(data: dict) -> str:
    """Auto-detect whether this is a request or state file."""
    if "action" in data and "status" not in data:
        return "request"
    if "status" in data and "action" not in data:
        return "state"
    if "action" in data and "status" in data:
        return "ambiguous"
    return "unknown"


# ── Request validation ────────────────────────────────────────────────


def validate_request(data: dict, filepath: str) -> list[str]:
    """Validate a queue task request file."""
    errors = []

    # Required fields
    for field in REQUIRED_REQUEST_FIELDS:
        if field not in data:
            errors.append(f"Missing required field: {field}")

    if errors:
        return errors

    # schema_version
    if data["schema_version"] != "1.0.0":
        errors.append(f"schema_version must be '1.0.0', got '{data['schema_version']}'")

    # task_id
    if not TASK_ID_PATTERN.match(str(data["task_id"])):
        errors.append(f"task_id must be 12-char hex, got '{data['task_id']}'")

    # action
    action = data["action"]
    if not isinstance(action, dict):
        errors.append(f"action must be object, got {type(action).__name__}")
        return errors

    for field in REQUIRED_ACTION_FIELDS:
        if field not in action:
            errors.append(f"action.{field} missing")

    if "service" in action and action["service"] not in VALID_SERVICES:
        errors.append(
            f"action.service must be one of {VALID_SERVICES}, got '{action['service']}'"
        )

    if "operation" in action and action["operation"] not in VALID_OPERATIONS:
        errors.append(
            f"action.operation must be one of {VALID_OPERATIONS}, got '{action['operation']}'"
        )

    # Extra action fields
    extra_action = set(action.keys()) - ALLOWED_ACTION_FIELDS
    if extra_action:
        errors.append(f"Unexpected action fields: {extra_action}")

    # priority
    if "priority" in data and data["priority"] not in VALID_PRIORITIES:
        errors.append(
            f"priority must be one of {VALID_PRIORITIES}, got '{data['priority']}'"
        )

    # max_attempts range
    if "max_attempts" in data:
        ma = data["max_attempts"]
        if not isinstance(ma, int) or ma < 1 or ma > 10:
            errors.append(f"max_attempts must be 1-10, got {ma}")

    # visibility_timeout_sec range
    if "visibility_timeout_sec" in data:
        vt = data["visibility_timeout_sec"]
        if not isinstance(vt, int) or vt < 30 or vt > 3600:
            errors.append(f"visibility_timeout_sec must be 30-3600, got {vt}")

    # Extra top-level fields
    extra = set(data.keys()) - ALLOWED_REQUEST_FIELDS
    if extra:
        errors.append(f"Unexpected fields: {extra}")

    return errors


# ── State validation ──────────────────────────────────────────────────


def validate_state(data: dict, filepath: str) -> list[str]:
    """Validate a queue task state file."""
    errors = []

    # Required fields
    for field in REQUIRED_STATE_FIELDS:
        if field not in data:
            errors.append(f"Missing required field: {field}")

    if errors:
        return errors

    # schema_version
    if data["schema_version"] != "1.0.0":
        errors.append(f"schema_version must be '1.0.0', got '{data['schema_version']}'")

    # task_id
    if not TASK_ID_PATTERN.match(str(data["task_id"])):
        errors.append(f"task_id must be 12-char hex, got '{data['task_id']}'")

    # status
    status = data["status"]
    if status not in VALID_STATUSES:
        errors.append(f"status must be one of {VALID_STATUSES}, got '{status}'")
        return errors

    # attempts
    attempts = data["attempts"]
    if not isinstance(attempts, int) or attempts < 0:
        errors.append(f"attempts must be non-negative integer, got {attempts}")

    # Lifecycle consistency checks
    if status == "done":
        if data.get("completed_at") is None:
            errors.append("status=done requires completed_at to be set")
        if data.get("result") is None:
            errors.append("status=done requires result to be set")

    if status == "dead":
        if data.get("dead_at") is None:
            errors.append("status=dead requires dead_at to be set")

    if status == "leased":
        if data.get("lease_owner") is None:
            errors.append("status=leased requires lease_owner to be set")
        if data.get("lease_until") is None:
            errors.append("status=leased requires lease_until to be set")

    # Validate result structure if present
    result = data.get("result")
    if result is not None:
        if not isinstance(result, dict):
            errors.append(f"result must be object, got {type(result).__name__}")
        else:
            for field in REQUIRED_RESULT_FIELDS:
                if field not in result:
                    errors.append(f"result.{field} missing")

    # Validate error structure if present
    error = data.get("error")
    if error is not None:
        if not isinstance(error, dict):
            errors.append(f"error must be object, got {type(error).__name__}")
        else:
            for field in REQUIRED_ERROR_FIELDS:
                if field not in error:
                    errors.append(f"error.{field} missing")

            if "type" in error and error["type"] not in VALID_ERROR_TYPES:
                errors.append(
                    f"error.type must be one of {VALID_ERROR_TYPES}, got '{error['type']}'"
                )

            if "code" in error and error["code"] not in VALID_ERROR_CODES:
                errors.append(
                    f"error.code must be one of {VALID_ERROR_CODES}, got '{error['code']}'"
                )

            # Consistency: error type matches code classification
            if "type" in error and "code" in error:
                etype = error["type"]
                ecode = error["code"]
                if etype == "transient" and ecode in PERMANENT_CODES:
                    errors.append(
                        f"error.type=transient but code '{ecode}' is permanent"
                    )
                if etype == "permanent" and ecode in TRANSIENT_CODES:
                    errors.append(
                        f"error.type=permanent but code '{ecode}' is transient"
                    )

    # Extra top-level fields
    extra = set(data.keys()) - ALLOWED_STATE_FIELDS
    if extra:
        errors.append(f"Unexpected fields: {extra}")

    return errors


# ── Lease expiry check ────────────────────────────────────────────────


def check_lease_expiry(data: dict, filepath: str) -> list[str]:
    """Check if a leased task has an expired lease (recovery scenario)."""
    info = []

    if data.get("status") != "leased":
        info.append(f"Not a leased task (status={data.get('status')}), skipping lease check")
        return info

    lease_until = data.get("lease_until")
    if not lease_until:
        info.append("WARN: status=leased but lease_until is null")
        return info

    try:
        lease_dt = datetime.fromisoformat(lease_until)
        if lease_dt.tzinfo is None:
            lease_dt = lease_dt.replace(tzinfo=timezone.utc)

        now = datetime.now(timezone.utc)

        if lease_dt < now:
            elapsed = now - lease_dt
            info.append(f"EXPIRED LEASE: lease_until={lease_until} ({elapsed} ago)")
            info.append(f"  lease_owner: {data.get('lease_owner')}")
            info.append(f"  attempts: {data.get('attempts')}")
            info.append(f"  Recovery: treat as pending, re-lease to next available worker")
        else:
            remaining = lease_dt - now
            info.append(f"ACTIVE LEASE: {remaining} remaining until {lease_until}")
            info.append(f"  lease_owner: {data.get('lease_owner')}")

    except (ValueError, TypeError) as e:
        info.append(f"WARN: Cannot parse lease_until '{lease_until}': {e}")

    return info


# ── File validation ───────────────────────────────────────────────────


def validate_file(
    filepath: str,
    force_type: str | None = None,
    check_lease: bool = False,
) -> bool:
    """Validate a single JSON file. Returns True if valid."""
    path = Path(filepath)

    if not path.exists():
        print(f"  FAIL  {filepath} (file not found)")
        return False

    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        print(f"  FAIL  {filepath} (invalid JSON: {e})")
        return False

    # Detect or force file type
    if force_type:
        file_type = force_type
    else:
        file_type = detect_file_type(data)

    if file_type == "unknown":
        print(f"  FAIL  {filepath} (cannot detect file type: no 'action' or 'status' field)")
        return False

    if file_type == "ambiguous":
        print(f"  FAIL  {filepath} (ambiguous: has both 'action' and 'status'. Use --request or --state)")
        return False

    # Validate
    if file_type == "request":
        errors = validate_request(data, filepath)
        label = "request"
    else:
        errors = validate_state(data, filepath)
        label = "state"

    if errors:
        print(f"  FAIL  {filepath} ({label})")
        for err in errors:
            print(f"        - {err}")
        return False

    print(f"  PASS  {filepath} ({label})")

    # Optional lease check
    if check_lease and file_type == "state":
        info = check_lease_expiry(data, filepath)
        for line in info:
            print(f"        {line}")

    return True


# ── Main ──────────────────────────────────────────────────────────────


def main():
    force_type = None
    check_lease = False
    files = []

    for arg in sys.argv[1:]:
        if arg == "--request":
            force_type = "request"
        elif arg == "--state":
            force_type = "state"
        elif arg == "--check-lease":
            check_lease = True
        else:
            files.append(arg)

    if not files:
        print("Usage: validate_queue_task.py [--request|--state] [--check-lease] <file.json> [...]")
        print()
        print("Options:")
        print("  --request      Force validation as request file")
        print("  --state        Force validation as state file")
        print("  --check-lease  Check lease expiry on state files")
        print()
        print("Auto-detects request vs state based on content if no flag given.")
        print()
        print("Examples:")
        print("  python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_request_valid.json")
        print("  python3 contracts/validate_queue_task.py --check-lease contracts/fixtures/queue_task_state_expired_lease.json")
        sys.exit(2)

    print(f"Validating {len(files)} file(s) against queue_task_contract v1.0.0")
    if force_type:
        print(f"Forced type: {force_type}")
    if check_lease:
        print("Lease expiry check: enabled")
    print()

    results = [validate_file(f, force_type, check_lease) for f in files]

    passed = sum(results)
    failed = len(results) - passed
    print()
    print(f"Results: {passed} passed, {failed} failed, {len(results)} total")

    sys.exit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
