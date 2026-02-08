#!/usr/bin/env python3
"""
Validate email daemon JSON output against the contract schema.

Usage:
    # Validate the bundled fixture
    python3 contracts/validate_daemon_output.py contracts/fixtures/email_daemon_sample.json

    # Validate real daemon output
    python3 contracts/validate_daemon_output.py ~/AI/openclaw-local/workspace/input/emails/*.json

    # Validate with Pydantic round-trip (requires arkai-gmail venv)
    PYTHONPATH=~/AI/arkai-gmail/src ~/AI/arkai-gmail/.venv/bin/python3 \
        contracts/validate_daemon_output.py --pydantic contracts/fixtures/email_daemon_sample.json

Exit codes:
    0 — All files valid
    1 — One or more files failed validation
    2 — Usage error (no files, missing contract)
"""

import json
import os
import re
import sys
from pathlib import Path

CONTRACT_PATH = Path(__file__).parent / "email_daemon_contract.json"

# Required top-level fields per contract
REQUIRED_FIELDS = {"schema_version", "processed_at", "email", "gate"}

# Required nested fields
REQUIRED_EMAIL_FIELDS = {"id", "message_id", "thread_id", "sender", "subject", "received_at"}
REQUIRED_GATE_FIELDS = {"result", "reason", "flagged"}
REQUIRED_CLASSIFICATION_FIELDS = {"category", "confidence", "summary", "reasoning"}
REQUIRED_ACTION_FIELDS = {"action", "reason"}
REQUIRED_VERDICT_FIELDS = {"decision"}

# Valid enum values
VALID_GATE_RESULTS = {"process", "skip", "flag"}
VALID_CATEGORIES = {"PRIORITY", "NEEDS_REPLY", "FYI", "NEWSLETTER", "RECEIPT", "SPAM_ISH"}
VALID_ACTIONS = {
    "add_label", "remove_label", "archive", "mark_read",
    "mark_unread", "star", "unstar", "create_draft", "snooze", "forward",
}
VALID_DECISIONS = {"APPROVE", "HUMAN_REVIEW", "BLOCK"}

EMAIL_ID_PATTERN = re.compile(r"^[a-f0-9]{12}$")


def validate_structure(data: dict, filepath: str) -> list[str]:
    """Validate JSON structure against contract schema. Returns list of errors."""
    errors = []

    # Top-level required fields
    for field in REQUIRED_FIELDS:
        if field not in data:
            errors.append(f"Missing required field: {field}")

    if errors:
        return errors  # Can't validate further without required fields

    # schema_version
    if data["schema_version"] != "1.0.0":
        errors.append(f"schema_version must be '1.0.0', got '{data['schema_version']}'")

    # email
    email = data["email"]
    for field in REQUIRED_EMAIL_FIELDS:
        if field not in email:
            errors.append(f"email.{field} missing")

    if "id" in email and not EMAIL_ID_PATTERN.match(email["id"]):
        errors.append(f"email.id must be 12-char hex, got '{email['id']}'")

    # gate
    gate = data["gate"]
    for field in REQUIRED_GATE_FIELDS:
        if field not in gate:
            errors.append(f"gate.{field} missing")

    if "result" in gate and gate["result"] not in VALID_GATE_RESULTS:
        errors.append(f"gate.result must be one of {VALID_GATE_RESULTS}, got '{gate['result']}'")

    if "flagged" in gate and not isinstance(gate["flagged"], bool):
        errors.append(f"gate.flagged must be boolean, got {type(gate['flagged']).__name__}")

    # classification (optional, null allowed)
    classification = data.get("classification")
    if classification is not None:
        for field in REQUIRED_CLASSIFICATION_FIELDS:
            if field not in classification:
                errors.append(f"classification.{field} missing")

        if "category" in classification and classification["category"] not in VALID_CATEGORIES:
            errors.append(
                f"classification.category must be one of {VALID_CATEGORIES}, "
                f"got '{classification['category']}'"
            )

        if "confidence" in classification:
            conf = classification["confidence"]
            if not isinstance(conf, (int, float)) or conf < 0 or conf > 1:
                errors.append(f"classification.confidence must be 0.0-1.0, got {conf}")

    # proposed_actions (optional array)
    actions = data.get("proposed_actions", [])
    if not isinstance(actions, list):
        errors.append(f"proposed_actions must be array, got {type(actions).__name__}")
    else:
        for i, action in enumerate(actions):
            for field in REQUIRED_ACTION_FIELDS:
                if field not in action:
                    errors.append(f"proposed_actions[{i}].{field} missing")
            if "action" in action and action["action"] not in VALID_ACTIONS:
                errors.append(
                    f"proposed_actions[{i}].action must be one of {VALID_ACTIONS}, "
                    f"got '{action['action']}'"
                )

    # critic_verdict (optional, null allowed)
    verdict = data.get("critic_verdict")
    if verdict is not None:
        for field in REQUIRED_VERDICT_FIELDS:
            if field not in verdict:
                errors.append(f"critic_verdict.{field} missing")
        if "decision" in verdict and verdict["decision"] not in VALID_DECISIONS:
            errors.append(
                f"critic_verdict.decision must be one of {VALID_DECISIONS}, "
                f"got '{verdict['decision']}'"
            )

    # No additional top-level properties
    allowed_top = {"schema_version", "processed_at", "email", "gate",
                   "classification", "proposed_actions", "critic_verdict"}
    extra = set(data.keys()) - allowed_top
    if extra:
        errors.append(f"Unexpected top-level fields: {extra}")

    return errors


def validate_pydantic_roundtrip(data: dict) -> list[str]:
    """Round-trip through Pydantic models from triage_daemon.py. Returns list of errors."""
    errors = []
    try:
        # These imports only work with arkai-gmail's venv + PYTHONPATH
        sys.path.insert(0, str(Path.home() / "AI" / "arkai-gmail" / "scripts"))
        from triage_daemon import DaemonTriageResult

        model = DaemonTriageResult.model_validate(data)
        roundtrip = json.loads(model.model_dump_json())

        # Verify key fields survive
        if roundtrip["email"]["id"] != data["email"]["id"]:
            errors.append("Round-trip: email.id changed")
        if roundtrip["schema_version"] != data["schema_version"]:
            errors.append("Round-trip: schema_version changed")
        if roundtrip.get("classification") != data.get("classification"):
            errors.append("Round-trip: classification changed")

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
        print("Usage: validate_daemon_output.py [--pydantic] <file.json> [file2.json ...]")
        print()
        print("Options:")
        print("  --pydantic  Also round-trip through Pydantic models (needs arkai-gmail venv)")
        print()
        print("Example:")
        print("  python3 contracts/validate_daemon_output.py contracts/fixtures/email_daemon_sample.json")
        sys.exit(2)

    if not CONTRACT_PATH.exists():
        print(f"WARNING: Contract not found at {CONTRACT_PATH} (using inline rules)")

    print(f"Validating {len(files)} file(s) against email_daemon_contract v1.0.0")
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
