"""Gmail API client for live email fetching.

Reuses authentication tokens from ~/.arkai-gmail/ (shared with arkai-gmail CLI).
"""

from pathlib import Path
from typing import Iterator

from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from googleapiclient.discovery import build, Resource

# Reuse arkai-gmail's token location
DEFAULT_CONFIG_DIR = Path.home() / ".arkai-gmail"
TOKEN_FILE = "token.json"

# Gmail API scopes (must match arkai-gmail)
SCOPES = [
    "https://www.googleapis.com/auth/gmail.readonly",
    "https://www.googleapis.com/auth/gmail.modify",
]


class GmailClient:
    """Minimal Gmail client that reuses arkai-gmail's authentication."""

    def __init__(self, config_dir: Path | None = None):
        self.config_dir = config_dir or DEFAULT_CONFIG_DIR
        self.token_path = self.config_dir / TOKEN_FILE
        self._service: Resource | None = None

    def is_authenticated(self) -> bool:
        """Check if we have valid authentication tokens."""
        return self.token_path.exists()

    def get_service(self) -> Resource:
        """Get authenticated Gmail API service.

        Raises:
            FileNotFoundError: If not authenticated via arkai-gmail
        """
        if self._service:
            return self._service

        if not self.token_path.exists():
            raise FileNotFoundError(
                f"Gmail token not found at {self.token_path}\n\n"
                "Run 'arkai-gmail auth' first to authenticate."
            )

        credentials = Credentials.from_authorized_user_file(
            str(self.token_path), SCOPES
        )

        # Refresh if expired
        if credentials.expired and credentials.refresh_token:
            credentials.refresh(Request())
            # Save refreshed token
            with open(self.token_path, "w") as f:
                f.write(credentials.to_json())

        self._service = build("gmail", "v1", credentials=credentials)
        return self._service

    def get_user_email(self) -> str:
        """Get authenticated user's email address."""
        service = self.get_service()
        profile = service.users().getProfile(userId="me").execute()
        return profile.get("emailAddress", "unknown")

    def search_messages(
        self,
        query: str,
        max_results: int = 10,
    ) -> list[dict]:
        """Search for messages matching query.

        Args:
            query: Gmail search query (e.g., "from:linkedin.com")
            max_results: Maximum messages to return

        Returns:
            List of message metadata dicts with 'id' and 'threadId'
        """
        service = self.get_service()
        results = service.users().messages().list(
            userId="me",
            q=query,
            maxResults=max_results,
        ).execute()

        return results.get("messages", [])

    def get_message(self, message_id: str) -> dict:
        """Fetch full message by ID.

        Args:
            message_id: Gmail message ID

        Returns:
            Full message dict (format="full")
        """
        service = self.get_service()
        return service.users().messages().get(
            userId="me",
            id=message_id,
            format="full",
        ).execute()

    def fetch_messages(
        self,
        query: str,
        max_results: int = 10,
    ) -> Iterator[dict]:
        """Search and fetch full messages.

        Args:
            query: Gmail search query
            max_results: Maximum messages to fetch

        Yields:
            Full message dicts
        """
        message_refs = self.search_messages(query, max_results)

        for ref in message_refs:
            yield self.get_message(ref["id"])
