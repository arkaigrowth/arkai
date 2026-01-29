"""JSON Schema validator for voice pipeline requests/results.

Validates data structures against schemas in contracts/ directory.
All requests and results should be validated before write.

Usage:
    from services.voice.validator import validate_request, validate_result

    # Validate a request
    try:
        validate_request({"id": "...", "action": "process", ...})
    except ValidationError as e:
        print(f"Invalid request: {e.message}")
"""

import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, ValidationError, validate

# Contracts directory is at repo root, relative to this file
# services/voice/validator.py -> ../../contracts/
CONTRACTS_DIR = Path(__file__).parent.parent.parent / "contracts"

# Cache loaded schemas to avoid repeated file reads
_schema_cache: dict[str, dict[str, Any]] = {}


def load_schema(name: str) -> dict[str, Any]:
    """Load a JSON Schema from the contracts directory.

    Args:
        name: Schema name without .schema.json suffix
              (e.g., "voice_request" loads "voice_request.schema.json")

    Returns:
        Parsed JSON Schema as dict

    Raises:
        FileNotFoundError: If schema file doesn't exist
        json.JSONDecodeError: If schema file is invalid JSON
    """
    if name in _schema_cache:
        return _schema_cache[name]

    schema_path = CONTRACTS_DIR / f"{name}.schema.json"
    schema = json.loads(schema_path.read_text())
    _schema_cache[name] = schema
    return schema


def validate_request(data: dict[str, Any]) -> None:
    """Validate a voice request against the schema.

    Args:
        data: Request data to validate

    Raises:
        ValidationError: If data doesn't match schema
    """
    schema = load_schema("voice_request")
    validate(data, schema, cls=Draft202012Validator)


def validate_result(data: dict[str, Any]) -> None:
    """Validate a voice result against the schema.

    Args:
        data: Result data to validate

    Raises:
        ValidationError: If data doesn't match schema
    """
    schema = load_schema("voice_result")
    validate(data, schema, cls=Draft202012Validator)


def is_valid_request(data: dict[str, Any]) -> bool:
    """Check if data is a valid voice request (non-throwing).

    Args:
        data: Request data to validate

    Returns:
        True if valid, False otherwise
    """
    try:
        validate_request(data)
        return True
    except ValidationError:
        return False


def is_valid_result(data: dict[str, Any]) -> bool:
    """Check if data is a valid voice result (non-throwing).

    Args:
        data: Result data to validate

    Returns:
        True if valid, False otherwise
    """
    try:
        validate_result(data)
        return True
    except ValidationError:
        return False


# Re-export ValidationError for convenience
__all__ = [
    "load_schema",
    "validate_request",
    "validate_result",
    "is_valid_request",
    "is_valid_result",
    "ValidationError",
]


if __name__ == "__main__":
    # Quick verification when run directly
    print(f"CONTRACTS_DIR: {CONTRACTS_DIR}")
    print(f"Exists: {CONTRACTS_DIR.exists()}")

    # Test loading schemas
    print("\nLoading schemas...")
    req_schema = load_schema("voice_request")
    print(f"voice_request.schema.json: loaded ({len(req_schema)} keys)")

    res_schema = load_schema("voice_result")
    print(f"voice_result.schema.json: loaded ({len(res_schema)} keys)")

    # Test validation with sample data
    print("\nTesting validation...")

    # Valid request
    valid_request = {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "action": "process",
        "params": {"limit": 5, "max_hours": 1.0},
        "requested_by": "alex",
        "requested_at": "2026-01-29T10:00:00Z",
    }

    try:
        validate_request(valid_request)
        print("Valid request: PASSED")
    except ValidationError as e:
        print(f"Valid request: FAILED - {e.message}")

    # Invalid request (missing required field)
    invalid_request = {"id": "test", "action": "process"}

    try:
        validate_request(invalid_request)
        print("Invalid request: SHOULD HAVE FAILED")
    except ValidationError:
        print("Invalid request: correctly rejected")

    print("\nAll checks passed!")
