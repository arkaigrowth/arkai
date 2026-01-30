"""
Tests for Gmail ingestion module.

Tests parse_gmail_message function which converts Gmail API JSON to EmailRecord.
"""

import base64
from datetime import datetime, timezone

import pytest

from arkai_inbox.ingestion import parse_gmail_message
from arkai_inbox.ingestion.gmail import (
    _decode_body_data,
    _detect_attachments,
    _detect_channel,
    _extract_body,
    _extract_headers,
    _parse_date,
    _parse_to_header,
)


def _b64encode(text: str) -> str:
    """Helper to base64url encode text for test fixtures."""
    return base64.urlsafe_b64encode(text.encode("utf-8")).decode("ascii")


class TestParseGmailMessage:
    """Tests for main parse_gmail_message function."""

    def test_basic_parsing(self):
        """Test parsing a simple Gmail message."""
        raw = {
            "id": "18d5a7b3c9e0f123",
            "threadId": "18d5a7b3c9e0f123",
            "labelIds": ["INBOX", "UNREAD"],
            "snippet": "Preview text...",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "Subject", "value": "Test Subject"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 -0500"},
                ],
                "body": {"data": _b64encode("Hello, this is the body.")},
            },
        }

        email = parse_gmail_message(raw)

        assert email.message_id == "18d5a7b3c9e0f123"
        assert email.thread_id == "18d5a7b3c9e0f123"
        assert email.from_header == "sender@example.com"
        assert email.to == ["user@example.com"]
        assert email.subject == "Test Subject"
        assert email.snippet == "Preview text..."
        assert email.text_body == "Hello, this is the body."
        assert email.html_body is None
        assert email.labels == ["INBOX", "UNREAD"]
        assert email.channel == "gmail"
        assert not email.has_attachments
        assert email.attachment_types == []

    def test_multipart_alternative(self):
        """Test parsing multipart/alternative (text + html)."""
        raw = {
            "id": "msg123",
            "threadId": "thread123",
            "labelIds": ["INBOX"],
            "snippet": "Preview",
            "payload": {
                "mimeType": "multipart/alternative",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 +0000"},
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "body": {"data": _b64encode("Plain text version")},
                    },
                    {
                        "mimeType": "text/html",
                        "body": {"data": _b64encode("<p>HTML version</p>")},
                    },
                ],
            },
        }

        email = parse_gmail_message(raw)

        assert email.text_body == "Plain text version"
        assert email.html_body == "<p>HTML version</p>"

    def test_missing_subject(self):
        """Test that subject can be None."""
        raw = {
            "id": "msg123",
            "threadId": "thread123",
            "labelIds": [],
            "snippet": "",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 +0000"},
                ],
                "body": {"data": _b64encode("No subject email")},
            },
        }

        email = parse_gmail_message(raw)
        assert email.subject is None

    def test_reply_to_header(self):
        """Test Reply-To header extraction."""
        raw = {
            "id": "msg123",
            "threadId": "thread123",
            "labelIds": [],
            "snippet": "",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "Reply-To", "value": "reply@different.com"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 +0000"},
                ],
                "body": {"data": _b64encode("Body")},
            },
        }

        email = parse_gmail_message(raw)
        assert email.reply_to == "reply@different.com"

    def test_missing_message_id_raises(self):
        """Test that missing id raises ValueError."""
        raw = {
            "threadId": "thread123",
            "payload": {"headers": []},
        }

        with pytest.raises(ValueError, match="Missing message id"):
            parse_gmail_message(raw)

    def test_missing_thread_id_raises(self):
        """Test that missing threadId raises ValueError."""
        raw = {
            "id": "msg123",
            "payload": {"headers": []},
        }

        with pytest.raises(ValueError, match="Missing threadId"):
            parse_gmail_message(raw)

    def test_missing_payload_raises(self):
        """Test that missing payload raises ValueError."""
        raw = {
            "id": "msg123",
            "threadId": "thread123",
        }

        with pytest.raises(ValueError, match="Missing payload"):
            parse_gmail_message(raw)

    def test_empty_json_raises(self):
        """Test that empty JSON raises ValueError."""
        with pytest.raises(ValueError, match="Empty message"):
            parse_gmail_message({})

    def test_raw_headers_preserved(self):
        """Test that all headers are stored in raw_headers."""
        raw = {
            "id": "msg123",
            "threadId": "thread123",
            "labelIds": [],
            "snippet": "",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "X-Custom-Header", "value": "custom-value"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 +0000"},
                ],
                "body": {"data": _b64encode("Body")},
            },
        }

        email = parse_gmail_message(raw)
        assert email.raw_headers["From"] == "sender@example.com"
        assert email.raw_headers["X-Custom-Header"] == "custom-value"


class TestHeaderExtraction:
    """Tests for header parsing functions."""

    def test_extract_headers_basic(self):
        """Test basic header extraction."""
        headers = [
            {"name": "From", "value": "sender@example.com"},
            {"name": "Subject", "value": "Test"},
        ]
        result = _extract_headers(headers)
        assert result == {"From": "sender@example.com", "Subject": "Test"}

    def test_extract_headers_empty(self):
        """Test empty headers list."""
        result = _extract_headers([])
        assert result == {}

    def test_extract_headers_preserves_case(self):
        """Test that header names preserve original case."""
        headers = [
            {"name": "Content-Type", "value": "text/plain"},
            {"name": "X-Custom-Header", "value": "value"},
        ]
        result = _extract_headers(headers)
        assert "Content-Type" in result
        assert "X-Custom-Header" in result

    def test_parse_to_single_address(self):
        """Test parsing single To address."""
        result = _parse_to_header("user@example.com")
        assert result == ["user@example.com"]

    def test_parse_to_multiple_addresses(self):
        """Test parsing multiple To addresses."""
        result = _parse_to_header("user1@example.com, user2@example.com")
        assert result == ["user1@example.com", "user2@example.com"]

    def test_parse_to_with_names(self):
        """Test parsing To addresses with display names."""
        result = _parse_to_header("John Doe <john@example.com>, Jane <jane@example.com>")
        assert len(result) == 2
        assert "John Doe <john@example.com>" in result

    def test_parse_to_empty(self):
        """Test parsing empty To header."""
        result = _parse_to_header("")
        assert result == []


class TestDateParsing:
    """Tests for date header parsing."""

    def test_parse_standard_rfc2822(self):
        """Test parsing standard RFC 2822 date."""
        result = _parse_date("Thu, 30 Jan 2026 10:00:00 -0500")
        assert result.year == 2026
        assert result.month == 1
        assert result.day == 30
        assert result.hour == 10

    def test_parse_utc_date(self):
        """Test parsing UTC date."""
        result = _parse_date("Thu, 30 Jan 2026 15:00:00 +0000")
        assert result.year == 2026
        assert result.tzinfo is not None

    def test_parse_malformed_date_fallback(self):
        """Test that malformed dates return current time."""
        result = _parse_date("not a date")
        # Should return some datetime (current time)
        assert isinstance(result, datetime)

    def test_parse_empty_date_fallback(self):
        """Test that empty date returns current time."""
        result = _parse_date("")
        assert isinstance(result, datetime)


class TestBase64Decoding:
    """Tests for base64 body decoding."""

    def test_decode_basic_text(self):
        """Test decoding basic text."""
        encoded = _b64encode("Hello, World!")
        result = _decode_body_data(encoded)
        assert result == "Hello, World!"

    def test_decode_unicode(self):
        """Test decoding Unicode text."""
        encoded = _b64encode("Hello, 世界! 🌍")
        result = _decode_body_data(encoded)
        assert result == "Hello, 世界! 🌍"

    def test_decode_empty_string(self):
        """Test decoding empty string."""
        result = _decode_body_data("")
        assert result == ""

    def test_decode_invalid_base64(self):
        """Test that invalid base64 returns empty string."""
        result = _decode_body_data("not valid base64!!!")
        assert result == ""

    def test_decode_url_safe_base64(self):
        """Test URL-safe base64 (- and _ instead of + and /)."""
        # Test data that would have + and / in standard base64
        text = "Test data with special chars: ?/+"
        encoded = base64.urlsafe_b64encode(text.encode()).decode()
        result = _decode_body_data(encoded)
        assert result == text


class TestBodyExtraction:
    """Tests for body extraction from payload."""

    def test_simple_text_body(self):
        """Test extracting simple text body."""
        payload = {
            "mimeType": "text/plain",
            "body": {"data": _b64encode("Simple text")},
        }
        html, text = _extract_body(payload)
        assert text == "Simple text"
        assert html is None

    def test_simple_html_body(self):
        """Test extracting simple HTML body."""
        payload = {
            "mimeType": "text/html",
            "body": {"data": _b64encode("<p>HTML</p>")},
        }
        html, text = _extract_body(payload)
        assert html == "<p>HTML</p>"
        assert text is None

    def test_multipart_alternative(self):
        """Test extracting multipart/alternative body."""
        payload = {
            "mimeType": "multipart/alternative",
            "parts": [
                {
                    "mimeType": "text/plain",
                    "body": {"data": _b64encode("Text version")},
                },
                {
                    "mimeType": "text/html",
                    "body": {"data": _b64encode("<p>HTML version</p>")},
                },
            ],
        }
        html, text = _extract_body(payload)
        assert text == "Text version"
        assert html == "<p>HTML version</p>"

    def test_nested_multipart(self):
        """Test extracting nested multipart body."""
        payload = {
            "mimeType": "multipart/mixed",
            "parts": [
                {
                    "mimeType": "multipart/alternative",
                    "parts": [
                        {
                            "mimeType": "text/plain",
                            "body": {"data": _b64encode("Nested text")},
                        },
                        {
                            "mimeType": "text/html",
                            "body": {"data": _b64encode("<p>Nested HTML</p>")},
                        },
                    ],
                },
            ],
        }
        html, text = _extract_body(payload)
        assert text == "Nested text"
        assert html == "<p>Nested HTML</p>"

    def test_empty_body(self):
        """Test empty payload body."""
        payload = {"mimeType": "text/plain", "body": {}}
        html, text = _extract_body(payload)
        assert html is None
        assert text is None


class TestAttachmentDetection:
    """Tests for attachment detection."""

    def test_no_attachments(self):
        """Test message with no attachments."""
        payload = {
            "mimeType": "text/plain",
            "body": {"data": _b64encode("No attachments")},
        }
        has_attachments, types = _detect_attachments(payload)
        assert not has_attachments
        assert types == []

    def test_pdf_attachment(self):
        """Test detecting PDF attachment."""
        payload = {
            "mimeType": "multipart/mixed",
            "parts": [
                {
                    "mimeType": "text/plain",
                    "body": {"data": _b64encode("Body text")},
                },
                {
                    "mimeType": "application/pdf",
                    "filename": "document.pdf",
                    "body": {"attachmentId": "attach123"},
                },
            ],
        }
        has_attachments, types = _detect_attachments(payload)
        assert has_attachments
        assert ".pdf" in types

    def test_multiple_attachments(self):
        """Test detecting multiple attachments."""
        payload = {
            "mimeType": "multipart/mixed",
            "parts": [
                {
                    "mimeType": "text/plain",
                    "body": {"data": _b64encode("Body")},
                },
                {
                    "mimeType": "application/pdf",
                    "filename": "doc.pdf",
                    "body": {"attachmentId": "a1"},
                },
                {
                    "mimeType": "image/jpeg",
                    "filename": "image.jpg",
                    "body": {"attachmentId": "a2"},
                },
            ],
        }
        has_attachments, types = _detect_attachments(payload)
        assert has_attachments
        assert ".pdf" in types
        assert ".jpg" in types
        assert len(types) == 2

    def test_attachment_by_content_disposition(self):
        """Test detecting attachment by Content-Disposition header."""
        payload = {
            "mimeType": "multipart/mixed",
            "parts": [
                {
                    "mimeType": "application/octet-stream",
                    "headers": [
                        {"name": "Content-Disposition", "value": "attachment; filename=data.bin"}
                    ],
                    "body": {"attachmentId": "a1"},
                },
            ],
        }
        has_attachments, types = _detect_attachments(payload)
        assert has_attachments

    def test_attachment_without_filename(self):
        """Test attachment with mime type but no filename."""
        payload = {
            "mimeType": "multipart/mixed",
            "parts": [
                {
                    "mimeType": "application/pdf",
                    "body": {"attachmentId": "a1"},
                },
            ],
        }
        has_attachments, types = _detect_attachments(payload)
        assert has_attachments
        assert "application/pdf" in types


class TestChannelDetection:
    """Tests for channel detection from From header."""

    def test_regular_gmail(self):
        """Test regular email defaults to gmail channel."""
        result = _detect_channel("sender@example.com")
        assert result == "gmail"

    def test_linkedin_notification(self):
        """Test LinkedIn notification detection."""
        result = _detect_channel("LinkedIn <notifications-noreply@linkedin.com>")
        assert result == "linkedin"

    def test_linkedin_email_domain(self):
        """Test linkedin-email.com domain."""
        result = _detect_channel("noreply@linkedin-email.com")
        assert result == "linkedin"

    def test_linkedin_e_subdomain(self):
        """Test e.linkedin.com domain."""
        result = _detect_channel("messages@e.linkedin.com")
        assert result == "linkedin"

    def test_linkedin_news_subdomain(self):
        """Test news.linkedin.com domain."""
        result = _detect_channel("digest@news.linkedin.com")
        assert result == "linkedin"

    def test_case_insensitive(self):
        """Test that domain matching is case-insensitive."""
        result = _detect_channel("LinkedIn <NOREPLY@LINKEDIN.COM>")
        assert result == "linkedin"

    def test_complex_from_header(self):
        """Test complex From header with display name."""
        result = _detect_channel('"LinkedIn Job Alerts" <jobalerts-noreply@linkedin.com>')
        assert result == "linkedin"

    def test_non_linkedin_similar_domain(self):
        """Test domain that looks similar but isn't LinkedIn."""
        result = _detect_channel("fake@not-linkedin.com")
        assert result == "gmail"


class TestIntegration:
    """Integration tests with realistic Gmail API responses."""

    def test_realistic_linkedin_email(self):
        """Test parsing realistic LinkedIn notification."""
        raw = {
            "id": "18d5a7b3c9e0f123",
            "threadId": "18d5a7b3c9e0f123",
            "labelIds": ["INBOX", "UNREAD", "CATEGORY_UPDATES"],
            "snippet": "Alex, you have new messages from John Doe and 2 others",
            "payload": {
                "mimeType": "multipart/alternative",
                "headers": [
                    {"name": "From", "value": "LinkedIn <notifications-noreply@linkedin.com>"},
                    {"name": "To", "value": "alex@example.com"},
                    {"name": "Subject", "value": "Alex, you have new messages"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 10:00:00 -0500"},
                    {"name": "Reply-To", "value": "noreply@linkedin.com"},
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "body": {"data": _b64encode("You have new messages. View them on LinkedIn.")},
                    },
                    {
                        "mimeType": "text/html",
                        "body": {"data": _b64encode("<html><body><p>You have new messages.</p></body></html>")},
                    },
                ],
            },
        }

        email = parse_gmail_message(raw)

        assert email.channel == "linkedin"
        assert email.from_header == "LinkedIn <notifications-noreply@linkedin.com>"
        assert email.subject == "Alex, you have new messages"
        assert email.text_body == "You have new messages. View them on LinkedIn."
        assert "<html>" in email.html_body
        assert email.reply_to == "noreply@linkedin.com"
        assert "CATEGORY_UPDATES" in email.labels

    def test_email_with_attachment(self):
        """Test parsing email with PDF attachment."""
        raw = {
            "id": "msg456",
            "threadId": "thread456",
            "labelIds": ["INBOX"],
            "snippet": "Please find attached the invoice",
            "payload": {
                "mimeType": "multipart/mixed",
                "headers": [
                    {"name": "From", "value": "billing@company.com"},
                    {"name": "To", "value": "user@example.com"},
                    {"name": "Subject", "value": "Invoice #12345"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 14:30:00 +0000"},
                ],
                "parts": [
                    {
                        "mimeType": "multipart/alternative",
                        "parts": [
                            {
                                "mimeType": "text/plain",
                                "body": {"data": _b64encode("Please find attached the invoice.")},
                            },
                            {
                                "mimeType": "text/html",
                                "body": {"data": _b64encode("<p>Please find attached the invoice.</p>")},
                            },
                        ],
                    },
                    {
                        "mimeType": "application/pdf",
                        "filename": "invoice_12345.pdf",
                        "body": {"attachmentId": "ANGjdJ8...", "size": 125000},
                    },
                ],
            },
        }

        email = parse_gmail_message(raw)

        assert email.has_attachments
        assert ".pdf" in email.attachment_types
        assert email.text_body == "Please find attached the invoice."
        assert email.subject == "Invoice #12345"

    def test_minimal_email(self):
        """Test parsing minimal valid email."""
        raw = {
            "id": "min123",
            "threadId": "min123",
            "payload": {
                "headers": [
                    {"name": "From", "value": "sender@test.com"},
                    {"name": "Date", "value": "Thu, 30 Jan 2026 12:00:00 +0000"},
                ],
                "body": {},
            },
        }

        email = parse_gmail_message(raw)

        assert email.message_id == "min123"
        assert email.from_header == "sender@test.com"
        assert email.to == []
        assert email.subject is None
        assert email.snippet == ""
        assert email.labels == []
