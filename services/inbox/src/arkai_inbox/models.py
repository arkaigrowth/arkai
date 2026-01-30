"""Integration contracts for arkai-inbox pipeline.

This module defines the data shapes that connect all components:
- EmailRecord: What ingestion produces (from Gmail API)
- create_evidence_bundle(): How Pre-Gate modules combine into CriticEvidenceBundle
- AuditEvent: What gets logged to JSONL

LOCKED: Changes here require updating all downstream consumers.
"""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Literal, Any
import uuid

from arkai_inbox.normalize import (
    normalize_for_risk_detection,
    extract_first_n_chars,
    extract_last_n_chars,
)
from arkai_inbox.url_extractor import extract_urls_from_html, ExtractedUrl
from arkai_inbox.quarantine import evaluate_hard_quarantine, QuarantineResult


# =============================================================================
# EmailRecord: What ingestion produces
# =============================================================================

@dataclass
class EmailRecord:
    """Internal representation of an email from Gmail API.

    This is what the ingestion layer produces and what all modules consume.
    Maps directly to Gmail API message format with format="full".

    Required fields come from Gmail API metadata.
    Optional fields may be missing depending on email type.
    """
    # Identity (from Gmail API)
    message_id: str  # Gmail API message ID (e.g., "18d5a7b3c9e0f123")
    thread_id: str   # Gmail API thread ID

    # Headers (extracted from Gmail payload.headers)
    from_header: str          # "LinkedIn <notifications-noreply@linkedin.com>"
    to: list[str]             # ["user@example.com"]
    subject: str | None       # May be None for some emails
    date: datetime            # Parsed from Date header
    reply_to: str | None = None  # Reply-To header if present

    # Content (from Gmail payload.body or payload.parts)
    snippet: str = ""         # Gmail's ~100 char preview
    html_body: str | None = None  # HTML part if present
    text_body: str | None = None  # Plain text part if present

    # Attachments (from Gmail payload.parts where filename exists)
    has_attachments: bool = False
    attachment_types: list[str] = field(default_factory=list)  # ["application/pdf", ".docx"]

    # Gmail metadata
    labels: list[str] = field(default_factory=list)  # ["INBOX", "UNREAD", "CATEGORY_UPDATES"]

    # Channel identification
    channel: Literal["gmail", "linkedin", "imessage", "telegram"] = "gmail"

    # Raw storage (for debugging/fixtures)
    raw_headers: dict[str, str] = field(default_factory=dict)  # All headers as key-value

    def get_body_for_parsing(self) -> str:
        """Return best available body content for parsing."""
        return self.html_body or self.text_body or self.snippet or ""


# =============================================================================
# CriticEvidenceBundle: What Pre-Gate produces for Critic
# =============================================================================

@dataclass
class CriticEvidenceBundle:
    """DETERMINISTIC evidence bundle computed by Pre-Gate.

    Reader CANNOT influence any field except proposed_action and proposed_reply_draft.
    All other fields are computed BEFORE Reader sees content.

    Matches contracts/critic_evidence_bundle.schema.json
    """
    # Message identity
    channel: Literal["gmail", "linkedin", "imessage", "telegram"]
    sender: str
    timestamp: datetime
    subject: str | None

    # DETERMINISTIC excerpts (Pre-Gate computes, not Reader)
    first_200_normalized: str
    last_200_normalized: str

    # Link analysis (HTML-parsed by url_extractor)
    link_domains: list[str]
    link_mismatch_flags: list[str]
    link_shortener_flags: list[str]

    # Attachments
    has_attachments: bool
    attachment_types: list[str]

    # Hard quarantine results
    quarantine_tier: Literal["PASS", "REVIEW", "QUARANTINE"]
    quarantine_reasons: list[str]

    # Soft auth score (sorting only) - placeholder for now
    auth_risk_score: float = 0.5
    auth_signals: dict[str, Any] = field(default_factory=dict)

    # Reader's proposed output (populated after Reader runs)
    proposed_action: str | None = None
    proposed_reply_draft: str | None = None


def create_evidence_bundle(email: EmailRecord) -> CriticEvidenceBundle:
    """Orchestrate Pre-Gate modules to create evidence bundle.

    This is the ONLY way to create a CriticEvidenceBundle.
    Ensures all fields are computed deterministically from the email.

    Args:
        email: EmailRecord from ingestion

    Returns:
        CriticEvidenceBundle with all Pre-Gate fields populated
    """
    # 1. Normalize content for excerpts
    body = email.get_body_for_parsing()
    normalized = normalize_for_risk_detection(body)
    first_200 = extract_first_n_chars(normalized, 200)
    last_200 = extract_last_n_chars(normalized, 200)

    # 2. Extract and analyze URLs
    urls: list[ExtractedUrl] = []
    if email.html_body:
        urls = extract_urls_from_html(email.html_body)

    link_domains = list(set(u.href_domain for u in urls if u.href_domain))
    link_mismatch_flags = [
        f"'{u.visible_text[:30]}' links to {u.href_domain} (text mentions {u.text_domain})"
        for u in urls if u.is_mismatch
    ]
    link_shortener_flags = [
        u.href_domain for u in urls if u.is_shortener
    ]

    # 3. Run hard quarantine rules
    quarantine_input = {
        'from': email.from_header,
        'reply_to': email.reply_to,
        'html_body': email.html_body,
    }
    quarantine_result: QuarantineResult = evaluate_hard_quarantine(quarantine_input)

    # 4. Build evidence bundle
    return CriticEvidenceBundle(
        channel=email.channel,
        sender=email.from_header,
        timestamp=email.date,
        subject=email.subject,
        first_200_normalized=first_200,
        last_200_normalized=last_200,
        link_domains=link_domains,
        link_mismatch_flags=link_mismatch_flags,
        link_shortener_flags=link_shortener_flags,
        has_attachments=email.has_attachments,
        attachment_types=email.attachment_types,
        quarantine_tier=quarantine_result.tier,
        quarantine_reasons=quarantine_result.reasons,
        # auth_risk_score and auth_signals left as defaults for now
        # Will be populated by auth_score.py when implemented
    )


# =============================================================================
# AuditEvent: What gets logged to JSONL
# =============================================================================

@dataclass
class AuditEvent:
    """Immutable event for JSONL audit log.

    Logged to ~/.arkai/runs/inbox-YYYY-MM-DD-HHMMSS/events.jsonl

    Each pipeline step appends one event. Events are append-only.
    """
    event_id: str  # UUID
    timestamp: datetime
    stage: Literal[
        "ingested",        # Email fetched from source
        "pre_gate",        # Pre-Gate analysis complete
        "quarantined",     # Hard quarantine triggered
        "reader_start",    # Reader LLM called
        "reader_complete", # Reader returned classification
        "critic_verdict",  # Critic approved/rejected/escalated
        "action_executed", # Actor performed action
        "skipped",         # User skipped this email
    ]
    message_id: str  # Gmail message ID
    channel: Literal["gmail", "linkedin", "imessage", "telegram"]

    # Stage-specific data
    quarantine_tier: str | None = None
    quarantine_reasons: list[str] | None = None
    link_domains: list[str] | None = None
    action: str | None = None  # "add_label:arkai/Priority", "archive", etc.
    draft_path: str | None = None  # Path to draft file if created
    error: str | None = None  # Error message if stage failed

    # Extra metadata
    metadata: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def create(
        cls,
        stage: str,
        message_id: str,
        channel: str = "gmail",
        **kwargs
    ) -> "AuditEvent":
        """Factory method to create audit event with auto-generated ID and timestamp."""
        return cls(
            event_id=str(uuid.uuid4()),
            timestamp=datetime.utcnow(),
            stage=stage,
            message_id=message_id,
            channel=channel,
            **kwargs
        )

    def to_jsonl_dict(self) -> dict:
        """Convert to dict for JSONL serialization."""
        d = {
            "event_id": self.event_id,
            "timestamp": self.timestamp.isoformat(),
            "stage": self.stage,
            "message_id": self.message_id,
            "channel": self.channel,
        }
        # Only include non-None optional fields
        if self.quarantine_tier:
            d["quarantine_tier"] = self.quarantine_tier
        if self.quarantine_reasons:
            d["quarantine_reasons"] = self.quarantine_reasons
        if self.link_domains:
            d["link_domains"] = self.link_domains
        if self.action:
            d["action"] = self.action
        if self.draft_path:
            d["draft_path"] = self.draft_path
        if self.error:
            d["error"] = self.error
        if self.metadata:
            d["metadata"] = self.metadata
        return d


# =============================================================================
# Integration Summary
# =============================================================================
#
# Pipeline flow:
#
#   Gmail API ──► EmailRecord ──► create_evidence_bundle() ──► CriticEvidenceBundle
#                     │                    │
#                     │                    ├── normalize.py (first_200, last_200)
#                     │                    ├── url_extractor.py (links, mismatches)
#                     │                    └── quarantine.py (tier, reasons)
#                     │
#                     └──► AuditEvent (logged at each stage)
#
# Data ownership:
# - Ingestion: Creates EmailRecord from Gmail API
# - Pre-Gate: Creates CriticEvidenceBundle via create_evidence_bundle()
# - Audit: Logs AuditEvent at each pipeline stage
# - Reader: Reads EmailRecord.body, writes to evidence.proposed_*
# - Critic: Reads CriticEvidenceBundle, approves/rejects/escalates
# - Actor: Executes approved actions
#
