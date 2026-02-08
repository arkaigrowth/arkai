#!/usr/bin/env python3
"""
Validate calendar daemon JSON output against the contract schema.

Usage:
    # Validate the bundled fixture
    python3 contracts/validate_calendar_output.py contracts/fixtures/calendar_daemon_sample.json

    # Validate real daemon output
    python3 contracts/validate_calendar_output.py ~/AI/openclaw-local/workspace/input/calendar/*.json

    # Validate with Pydantic round-trip (requires arkai-gmail venv)
    ~/AI/arkai-gmail/.venv/bin/python3 \
        contracts/validate_calendar_output.py --pydantic contracts/fixtures/calendar_daemon_sample.json

Exit codes:
    0 — All files valid
    1 — One or more files failed validation
    2 — Usage error (no files, missing contract)
"""

import json
import re
import sys
from pathlib import Path

# Required top-level fields per contract
REQUIRED_FIELDS = {"schema_version", "processed_at", "event"}

# Required event fields
REQUIRED_EVENT_FIELDS = {
    "id", "google_event_id", "calendar_id", "calendar_name",
    "status", "summary", "start", "end",
}

# Required attendee fields
REQUIRED_ATTENDEE_FIELDS = {"email", "response_status"}

# Valid enum values
VALID_STATUSES = {"confirmed", "tentative", "cancelled"}
VALID_RESPONSE_STATUSES = {"needsAction", "declined", "tentative", "accepted"}

EVENT_ID_PATTERN = re.compile(r"^[a-f0-9]{12}$")


def validate_structure(data: dict, filepath: str) -> list[str]:
    """Validate JSON structure against contract schema. Returns list of errors."""
    errors = []

    # Top-level required fields
    for field in REQUIRED_FIELDS:
        if field not in data:
            errors.append(f"Missing required field: {field}")

    if errors:
        return errors

    # schema_version
    if data["schema_version"] != "1.0.0":
        errors.append(f"schema_version must be '1.0.0', got '{data['schema_version']}'")

    # event
    event = data["event"]
    if not isinstance(event, dict):
        errors.append(f"event must be object, got {type(event).__name__}")
        return errors

    for field in REQUIRED_EVENT_FIELDS:
        if field not in event:
            errors.append(f"event.{field} missing")

    if "id" in event and not EVENT_ID_PATTERN.match(str(event["id"])):
        errors.append(f"event.id must be 12-char hex, got '{event['id']}'")

    if "status" in event and event["status"] not in VALID_STATUSES:
        errors.append(
            f"event.status must be one of {VALID_STATUSES}, got '{event['status']}'"
        )

    # attendees (optional array)
    attendees = event.get("attendees", [])
    if not isinstance(attendees, list):
        errors.append(f"event.attendees must be array, got {type(attendees).__name__}")
    else:
        for i, att in enumerate(attendees):
            for field in REQUIRED_ATTENDEE_FIELDS:
                if field not in att:
                    errors.append(f"event.attendees[{i}].{field} missing")
            if "response_status" in att and att["response_status"] not in VALID_RESPONSE_STATUSES:
                errors.append(
                    f"event.attendees[{i}].response_status must be one of "
                    f"{VALID_RESPONSE_STATUSES}, got '{att['response_status']}'"
                )

    # No additional top-level properties
    allowed_top = {"schema_version", "processed_at", "event"}
    extra = set(data.keys()) - allowed_top
    if extra:
        errors.append(f"Unexpected top-level fields: {extra}")

    return errors


def validate_pydantic_roundtrip(data: dict) -> list[str]:
    """Round-trip through Pydantic models from calendar_daemon.py."""
    errors = []
    try:
        sys.path.insert(0, str(Path.home() / "AI" / "arkai-gmail" / "scripts"))
        from calendar_daemon import CalendarDaemonResult

        model = CalendarDaemonResult.model_validate(data)
        roundtrip = json.loads(model.model_dump_json(by_alias=True))

        if roundtrip["event"]["id"] != data["event"]["id"]:
            errors.append("Round-trip: event.id changed")
        if roundtrip["schema_version"] != data["schema_version"]:
            errors.append("Round-trip: schema_version changed")
        if roundtrip["event"]["summary"] != data["event"]["summary"]:
            errors.append("Round-trip: event.summary changed")

    except ImportError as e:
        errors.append(f"Pydantic round-trip skipped (import error: {e}). Use arkai-gmail venv.")
    except Exception as e:
        errors.append(f"Pydantic round-trip failed: {e}")

    return errors


def validate_file(filepath: str, use_pydantic: bool = False) -> bool:
    """Validate a single JSON file. Returns True if valid."""
    path = Path(filepath)

    if not path.exists():
        print(f"  FAIL  {filepath} (file not found)")
        return False

    # Skip the index file
    if path.name == "upcoming_index.json":
        print(f"  SKIP  {filepath} (index file, not per-event)")
        return True

    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        print(f"  FAIL  {filepath} (invalid JSON: {e})")
        return False

    errors = validate_structure(data, filepath)

    if use_pydantic and not errors:
        errors.extend(validate_pydantic_roundtrip(data))

    if errors:
        print(f"  FAIL  {filepath}")
        for err in errors:
            print(f"        - {err}")
        return False

    print(f"  PASS  {filepath}")
    return True


def main():
    use_pydantic = "--pydantic" in sys.argv
    files = [f for f in sys.argv[1:] if f != "--pydantic"]

    if not files:
        print("Usage: validate_calendar_output.py [--pydantic] <file.json> [file2.json ...]")
        print()
        print("Options:")
        print("  --pydantic  Also round-trip through Pydantic models (needs arkai-gmail venv)")
        print()
        print("Example:")
        print("  python3 contracts/validate_calendar_output.py contracts/fixtures/calendar_daemon_sample.json")
        sys.exit(2)

    print(f"Validating {len(files)} file(s) against calendar_daemon_contract v1.0.0")
    if use_pydantic:
        print("Pydantic round-trip: enabled")
    print()

    results = [validate_file(f, use_pydantic) for f in files]

    passed = sum(results)
    failed = len(results) - passed
    print()
    print(f"Results: {passed} passed, {failed} failed, {len(results)} total")

    sys.exit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
