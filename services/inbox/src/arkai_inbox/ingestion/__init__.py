"""Gmail ingestion module for arkai-inbox.

Parses raw Gmail API responses into EmailRecord objects.

Usage:
    from arkai_inbox.ingestion import parse_gmail_message

    raw = gmail_api.messages().get(userId='me', id='...', format='full').execute()
    email = parse_gmail_message(raw)
"""

from arkai_inbox.ingestion.gmail import parse_gmail_message

__all__ = ["parse_gmail_message"]
