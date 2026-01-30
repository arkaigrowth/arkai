# Inbox Pipeline Test Fixtures

Test fixtures for the inbox phishing detection pipeline.

## Directory Structure

```
fixtures/
├── linkedin_real/     # Real LinkedIn emails exported from Gmail
│   └── .gitkeep
├── linkedin_spoof/    # Synthetic phishing examples
│   ├── spoof_link_mismatch.json
│   └── spoof_reply_to.json
└── README.md
```

## Adding Real Fixtures

Export real LinkedIn emails using arkai-gmail:

```bash
# Export a specific message by ID
arkai-gmail export -m <message_id> -o fixtures/linkedin_real/msg1.json

# Find LinkedIn message IDs first
arkai-gmail search --query "from:linkedin.com" --limit 10
```

Real fixtures provide baseline examples of legitimate LinkedIn emails for comparison testing.

## Spoof Fixtures

### spoof_link_mismatch.json

**Attack Vector:** Link text/href mismatch (display text spoofing)

- **From:** `LinkedIn <notifications-noreply@linkedin.com>` (appears legitimate)
- **Subject:** "You have a new connection request"
- **Payload:** HTML contains `<a href="https://evil-linkedin.ru/confirm">linkedin.com/confirm</a>`

The visible link text shows `linkedin.com/confirm` but the actual href points to `evil-linkedin.ru`. This is a classic phishing technique where users see a trusted domain but clicking navigates to a malicious site.

**Detection Strategy:** Compare link text against href domain. Flag when:
- Link text contains a domain name (e.g., `linkedin.com`, `google.com`)
- That domain differs from the href's actual domain

### spoof_reply_to.json

**Attack Vector:** Reply-To header mismatch

- **From:** `LinkedIn <messages-noreply@linkedin.com>` (appears legitimate)
- **Reply-To:** `phisher@evil.com` (DIFFERENT from From domain)
- **Subject:** "Someone sent you a message"
- **Payload:** HTML body is innocuous (links go to real linkedin.com)

The email appears to be from LinkedIn but replies go to an attacker's address. Users who hit "reply" would unknowingly send their response to the phisher. This technique bypasses link-based detection since all visible links are legitimate.

**Detection Strategy:** Compare From domain against Reply-To domain. Flag when:
- Reply-To header exists
- Reply-To domain differs from From domain
- Especially suspicious when From is a known brand (linkedin.com, google.com, etc.)

## Gmail API Format

All fixtures use Gmail API format (format="full"):

```json
{
  "id": "message_id",
  "threadId": "thread_id",
  "labelIds": ["INBOX", "UNREAD", ...],
  "snippet": "Preview text...",
  "historyId": "12345678",
  "internalDate": "1706540400000",
  "sizeEstimate": 4521,
  "payload": {
    "mimeType": "multipart/alternative",
    "headers": [
      {"name": "From", "value": "..."},
      {"name": "Subject", "value": "..."},
      ...
    ],
    "parts": [
      {
        "mimeType": "text/plain",
        "body": {"data": "base64_encoded_content"}
      },
      {
        "mimeType": "text/html",
        "body": {"data": "base64_encoded_content"}
      }
    ]
  }
}
```

Body data is base64-encoded (URL-safe variant). Decode with:

```python
import base64
decoded = base64.urlsafe_b64decode(body_data).decode('utf-8')
```

## Usage in Tests

```python
import json
from pathlib import Path

FIXTURES = Path(__file__).parent / "fixtures"

def test_link_mismatch_detection():
    with open(FIXTURES / "linkedin_spoof/spoof_link_mismatch.json") as f:
        msg = json.load(f)

    result = detect_phishing(msg)
    assert result.is_suspicious
    assert "link_mismatch" in result.flags

def test_reply_to_mismatch_detection():
    with open(FIXTURES / "linkedin_spoof/spoof_reply_to.json") as f:
        msg = json.load(f)

    result = detect_phishing(msg)
    assert result.is_suspicious
    assert "reply_to_mismatch" in result.flags
```
