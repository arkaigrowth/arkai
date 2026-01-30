"""Gmail API JSON parser for EmailRecord conversion.

This module parses raw Gmail API message JSON (format="full") into EmailRecord objects.
Handles header extraction, base64 body decoding, multipart handling, and attachment detection.

Usage:
    from arkai_inbox.ingestion.gmail import parse_gmail_message

    raw = gmail_api.messages().get(userId='me', id='...', format='full').execute()
    email = parse_gmail_message(raw)
"""

import base64
from datetime import datetime
from email.utils import parsedate_to_datetime
from typing import Any

from arkai_inbox.models import EmailRecord


# Known LinkedIn email domains for channel detection
LINKEDIN_DOMAINS = frozenset([
    "linkedin.com",
    "linkedin-email.com",
    "e.linkedin.com",
    "news.linkedin.com",
])


def parse_gmail_message(raw_json: dict[str, Any]) -> EmailRecord:
    """Parse Gmail API message JSON into EmailRecord.

    Args:
        raw_json: Full Gmail API message response (format="full")

    Returns:
        EmailRecord populated from the message

    Raises:
        ValueError: If required fields (id, threadId, payload) are missing
    """
    if not raw_json:
        raise ValueError("Empty message JSON")

    message_id = raw_json.get("id")
    thread_id = raw_json.get("threadId")

    if not message_id:
        raise ValueError("Missing message id")
    if not thread_id:
        raise ValueError("Missing threadId")

    payload = raw_json.get("payload")
    if not payload:
        raise ValueError("Missing payload")

    # Extract headers as dict
    raw_headers = _extract_headers(payload.get("headers", []))

    # Parse individual headers
    from_header = raw_headers.get("From", raw_headers.get("from", ""))
    to_raw = raw_headers.get("To", raw_headers.get("to", ""))
    to_list = _parse_to_header(to_raw)
    subject = raw_headers.get("Subject", raw_headers.get("subject"))
    date_str = raw_headers.get("Date", raw_headers.get("date", ""))
    reply_to = raw_headers.get("Reply-To", raw_headers.get("reply-to"))

    # Parse date
    date = _parse_date(date_str)

    # Extract body content
    html_body, text_body = _extract_body(payload)

    # Detect attachments
    has_attachments, attachment_types = _detect_attachments(payload)

    # Detect channel from sender domain
    channel = _detect_channel(from_header)

    return EmailRecord(
        message_id=message_id,
        thread_id=thread_id,
        from_header=from_header,
        to=to_list,
        subject=subject,
        date=date,
        reply_to=reply_to,
        snippet=raw_json.get("snippet", ""),
        html_body=html_body,
        text_body=text_body,
        has_attachments=has_attachments,
        attachment_types=attachment_types,
        labels=raw_json.get("labelIds", []),
        channel=channel,
        raw_headers=raw_headers,
    )


def _extract_headers(headers: list[dict[str, str]]) -> dict[str, str]:
    """Convert Gmail API headers array to dict.

    Gmail API returns headers as: [{"name": "From", "value": "..."}]
    Converts to: {"From": "..."}

    Preserves original case for header names.
    """
    result = {}
    for header in headers:
        name = header.get("name", "")
        value = header.get("value", "")
        if name:
            result[name] = value
    return result


def _parse_to_header(to_raw: str) -> list[str]:
    """Parse To header into list of addresses.

    Handles:
    - Single address: "user@example.com"
    - Multiple addresses: "user1@example.com, user2@example.com"
    - Names with addresses: "John Doe <john@example.com>"
    """
    if not to_raw:
        return []

    # Split by comma, strip whitespace
    addresses = [addr.strip() for addr in to_raw.split(",")]
    return [addr for addr in addresses if addr]


def _parse_date(date_str: str) -> datetime:
    """Parse email Date header into datetime.

    Uses email.utils.parsedate_to_datetime which handles RFC 2822 dates.
    Falls back to current UTC time if parsing fails.
    """
    if not date_str:
        return datetime.utcnow()

    try:
        return parsedate_to_datetime(date_str)
    except (ValueError, TypeError):
        # Malformed date, return current time
        return datetime.utcnow()


def _extract_body(payload: dict[str, Any]) -> tuple[str | None, str | None]:
    """Extract HTML and plain text body from payload.

    Handles:
    - Simple messages with body.data in payload
    - multipart/alternative with text/plain and text/html parts
    - Nested multipart structures

    Returns:
        Tuple of (html_body, text_body), either can be None
    """
    html_body = None
    text_body = None

    mime_type = payload.get("mimeType", "")

    # Check if this payload has direct body data
    body = payload.get("body", {})
    body_data = body.get("data")

    if body_data and not payload.get("parts"):
        # Simple message with body directly in payload
        decoded = _decode_body_data(body_data)
        if "text/html" in mime_type:
            html_body = decoded
        elif "text/plain" in mime_type:
            text_body = decoded
        else:
            # Default to text if mime type unclear
            text_body = decoded

    # Check for multipart structure
    parts = payload.get("parts", [])
    for part in parts:
        part_mime = part.get("mimeType", "")
        part_body = part.get("body", {})
        part_data = part_body.get("data")

        # Skip parts with attachments (they have non-empty filename or attachmentId)
        if part_body.get("attachmentId") or part.get("filename"):
            continue

        if part_data:
            decoded = _decode_body_data(part_data)
            if "text/html" in part_mime:
                html_body = decoded
            elif "text/plain" in part_mime:
                text_body = decoded

        # Recurse into nested multipart
        nested_parts = part.get("parts", [])
        if nested_parts:
            nested_html, nested_text = _extract_body(part)
            if nested_html and not html_body:
                html_body = nested_html
            if nested_text and not text_body:
                text_body = nested_text

    return html_body, text_body


def _decode_body_data(data: str) -> str:
    """Decode base64url-encoded body data.

    Gmail API uses URL-safe base64 encoding (no padding).
    """
    if not data:
        return ""

    try:
        # base64url decode (Gmail uses URL-safe variant)
        decoded_bytes = base64.urlsafe_b64decode(data)
        return decoded_bytes.decode("utf-8", errors="replace")
    except Exception:
        return ""


def _detect_attachments(payload: dict[str, Any]) -> tuple[bool, list[str]]:
    """Detect attachments in message payload.

    Looks for:
    - Parts with filename attribute
    - Parts with attachmentId in body
    - Content-Disposition: attachment

    Returns:
        Tuple of (has_attachments, attachment_types)
    """
    attachment_types: list[str] = []

    def scan_parts(parts: list[dict[str, Any]]) -> None:
        for part in parts:
            mime_type = part.get("mimeType", "")
            body = part.get("body", {})
            filename = part.get("filename", "")

            # Check if this is an attachment
            is_attachment = False

            # Has filename = attachment
            if filename:
                is_attachment = True

            # Has attachmentId = attachment
            if body.get("attachmentId"):
                is_attachment = True

            # Check Content-Disposition header
            headers = part.get("headers", [])
            for header in headers:
                if header.get("name", "").lower() == "content-disposition":
                    if "attachment" in header.get("value", "").lower():
                        is_attachment = True

            if is_attachment:
                # Record the type - prefer filename extension, fallback to mime type
                if filename and "." in filename:
                    ext = "." + filename.rsplit(".", 1)[-1].lower()
                    attachment_types.append(ext)
                elif mime_type and mime_type not in ("text/plain", "text/html"):
                    attachment_types.append(mime_type)

            # Recurse into nested parts
            nested = part.get("parts", [])
            if nested:
                scan_parts(nested)

    parts = payload.get("parts", [])
    scan_parts(parts)

    return bool(attachment_types), attachment_types


def _detect_channel(from_header: str) -> str:
    """Detect message channel from From header domain.

    Checks if sender domain matches known providers:
    - linkedin.com, e.linkedin.com -> "linkedin"
    - Default -> "gmail"

    Returns:
        Channel identifier
    """
    # Extract domain from From header
    # Format: "Name <email@domain.com>" or "email@domain.com"
    from_lower = from_header.lower()

    # Try to extract domain
    domain = ""
    if "@" in from_lower:
        # Get part after @ and before >
        at_idx = from_lower.rfind("@")
        rest = from_lower[at_idx + 1:]
        # Remove trailing > if present
        if ">" in rest:
            domain = rest[:rest.index(">")]
        else:
            domain = rest.strip()

    # Check against known domains
    if domain in LINKEDIN_DOMAINS:
        return "linkedin"

    # Check subdomain matches (e.g., mail.linkedin.com)
    for known_domain in LINKEDIN_DOMAINS:
        if domain.endswith("." + known_domain):
            return "linkedin"

    return "gmail"
