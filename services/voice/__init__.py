"""Voice pipeline services for arkai.

This package contains VPS-side voice processing components:
- paths: Canonical path definitions
- validator: JSON Schema validation for requests/results
"""

from .paths import (
    VPS_ARTIFACTS,
    VPS_REQUESTS,
    VPS_RESULTS,
    VPS_AUDIO_CACHE,
    VPS_AUDIT_LOG,
    TELEGRAM_INBOUND,
)

__all__ = [
    "VPS_ARTIFACTS",
    "VPS_REQUESTS",
    "VPS_RESULTS",
    "VPS_AUDIO_CACHE",
    "VPS_AUDIT_LOG",
    "TELEGRAM_INBOUND",
]
