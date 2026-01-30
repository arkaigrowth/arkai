"""Audit logging for inbox pipeline.

MVP implementation: append-only JSONL event log.

Storage: ~/.arkai/runs/inbox-{timestamp}/events.jsonl
"""

import json
from datetime import datetime
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from arkai_inbox.models import AuditEvent


class AuditLog:
    """Append-only JSONL audit log for a single pipeline run.

    Usage:
        audit = AuditLog.create_run()
        audit.log(AuditEvent.create("ingested", message_id="abc123"))
        audit.log(AuditEvent.create("pre_gate", message_id="abc123", quarantine_tier="PASS"))
    """

    def __init__(self, run_dir: Path):
        """Initialize audit log for a specific run directory.

        Args:
            run_dir: Directory for this run (created if doesn't exist)
        """
        self.run_dir = run_dir
        self.events_file = run_dir / "events.jsonl"
        self._ensure_dir()

    def _ensure_dir(self) -> None:
        """Create run directory if it doesn't exist."""
        self.run_dir.mkdir(parents=True, exist_ok=True)

    @classmethod
    def create_run(cls, base_dir: Path | None = None) -> "AuditLog":
        """Create a new audit log for a pipeline run.

        Args:
            base_dir: Base directory for runs (default: ~/.arkai/runs/)

        Returns:
            AuditLog instance for the new run
        """
        if base_dir is None:
            base_dir = Path.home() / ".arkai" / "runs"

        timestamp = datetime.utcnow().strftime("%Y-%m-%d-%H%M%S")
        run_dir = base_dir / f"inbox-{timestamp}"

        return cls(run_dir)

    @classmethod
    def from_existing(cls, run_dir: Path) -> "AuditLog":
        """Load an existing audit log.

        Args:
            run_dir: Path to existing run directory

        Returns:
            AuditLog instance

        Raises:
            FileNotFoundError: If run_dir doesn't exist
        """
        if not run_dir.exists():
            raise FileNotFoundError(f"Run directory not found: {run_dir}")
        return cls(run_dir)

    def log(self, event: "AuditEvent") -> None:
        """Append an event to the audit log.

        Args:
            event: AuditEvent to log
        """
        line = json.dumps(event.to_jsonl_dict(), ensure_ascii=False)
        with open(self.events_file, "a", encoding="utf-8") as f:
            f.write(line + "\n")

    def read_events(self) -> list[dict]:
        """Read all events from the audit log.

        Returns:
            List of event dicts (most recent last)
        """
        if not self.events_file.exists():
            return []

        events = []
        with open(self.events_file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    events.append(json.loads(line))
        return events

    def get_run_summary(self) -> dict:
        """Get summary statistics for this run.

        Returns:
            Dict with counts by stage, quarantine tier, etc.
        """
        events = self.read_events()

        summary = {
            "run_dir": str(self.run_dir),
            "total_events": len(events),
            "by_stage": {},
            "by_tier": {"PASS": 0, "REVIEW": 0, "QUARANTINE": 0},
            "errors": 0,
        }

        for event in events:
            stage = event.get("stage", "unknown")
            summary["by_stage"][stage] = summary["by_stage"].get(stage, 0) + 1

            if event.get("quarantine_tier"):
                tier = event["quarantine_tier"]
                if tier in summary["by_tier"]:
                    summary["by_tier"][tier] += 1

            if event.get("error"):
                summary["errors"] += 1

        return summary


# Convenience functions for common logging patterns

def log_ingested(audit: AuditLog, message_id: str, channel: str = "gmail") -> None:
    """Log that a message was ingested."""
    from arkai_inbox.models import AuditEvent
    audit.log(AuditEvent.create("ingested", message_id=message_id, channel=channel))


def log_pre_gate(
    audit: AuditLog,
    message_id: str,
    quarantine_tier: str,
    quarantine_reasons: list[str],
    link_domains: list[str],
    channel: str = "gmail",
) -> None:
    """Log Pre-Gate analysis results."""
    from arkai_inbox.models import AuditEvent
    audit.log(AuditEvent.create(
        "pre_gate",
        message_id=message_id,
        channel=channel,
        quarantine_tier=quarantine_tier,
        quarantine_reasons=quarantine_reasons,
        link_domains=link_domains,
    ))


def log_quarantined(
    audit: AuditLog,
    message_id: str,
    reasons: list[str],
    channel: str = "gmail",
) -> None:
    """Log that a message was quarantined."""
    from arkai_inbox.models import AuditEvent
    audit.log(AuditEvent.create(
        "quarantined",
        message_id=message_id,
        channel=channel,
        quarantine_tier="QUARANTINE",
        quarantine_reasons=reasons,
    ))


def log_action(
    audit: AuditLog,
    message_id: str,
    action: str,
    draft_path: str | None = None,
    channel: str = "gmail",
) -> None:
    """Log an executed action."""
    from arkai_inbox.models import AuditEvent
    audit.log(AuditEvent.create(
        "action_executed",
        message_id=message_id,
        channel=channel,
        action=action,
        draft_path=draft_path,
    ))


def log_error(
    audit: AuditLog,
    message_id: str,
    stage: str,
    error: str,
    channel: str = "gmail",
) -> None:
    """Log an error at any stage."""
    from arkai_inbox.models import AuditEvent
    audit.log(AuditEvent.create(
        stage,
        message_id=message_id,
        channel=channel,
        error=error,
    ))
