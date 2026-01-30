"""
Pre-Gate normalization module for deterministic risk pattern detection.

Transforms raw email content into a canonical form for consistent risk analysis.
"""

import re
import unicodedata
from typing import Optional

from bs4 import BeautifulSoup


def html_to_text(html: str) -> str:
    """
    Convert HTML to plain text using BeautifulSoup.

    Args:
        html: Raw HTML string

    Returns:
        Plain text with HTML tags removed
    """
    if not html:
        return ""

    soup = BeautifulSoup(html, "lxml")

    # Remove script and style elements
    for script_or_style in soup(["script", "style"]):
        script_or_style.decompose()

    # Get text and handle whitespace
    text = soup.get_text(separator=" ")
    return text


def strip_zero_width_chars(text: str) -> str:
    """
    Remove zero-width and other invisible Unicode characters.

    Strips:
    - \u200b: Zero-width space
    - \u200c: Zero-width non-joiner
    - \u200d: Zero-width joiner
    - \ufeff: Zero-width no-break space (BOM)
    - \u00ad: Soft hyphen

    Args:
        text: Input text

    Returns:
        Text with zero-width characters removed
    """
    zero_width_chars = [
        "\u200b",  # Zero-width space
        "\u200c",  # Zero-width non-joiner
        "\u200d",  # Zero-width joiner
        "\ufeff",  # Zero-width no-break space (BOM)
        "\u00ad",  # Soft hyphen
    ]

    for char in zero_width_chars:
        text = text.replace(char, "")

    return text


def collapse_whitespace(text: str) -> str:
    """
    Collapse multiple whitespace characters into single spaces.

    Converts multiple spaces, tabs, newlines, etc. into single space.
    Also strips leading/trailing whitespace.

    Args:
        text: Input text

    Returns:
        Text with normalized whitespace
    """
    # Replace multiple whitespace chars with single space
    text = re.sub(r"\s+", " ", text)
    # Strip leading/trailing whitespace
    return text.strip()


def extract_first_n_chars(text: str, n: int = 200) -> str:
    """
    Extract first N characters from text for evidence bundle.

    Args:
        text: Input text
        n: Number of characters to extract (default: 200)

    Returns:
        First N characters of text
    """
    if not text:
        return ""
    return text[:n]


def extract_last_n_chars(text: str, n: int = 200) -> str:
    """
    Extract last N characters from text for evidence bundle.

    Args:
        text: Input text
        n: Number of characters to extract (default: 200)

    Returns:
        Last N characters of text
    """
    if not text:
        return ""
    if len(text) <= n:
        return text
    return text[-n:]


def normalize_for_risk_detection(raw_content: str) -> str:
    """
    Normalize content for deterministic risk pattern detection.

    Applies a series of transformations to produce canonical form:
    1. Convert HTML to plain text
    2. Unicode normalize (NFKC)
    3. Strip zero-width characters
    4. Collapse whitespace
    5. Lowercase

    Args:
        raw_content: Raw email content (may contain HTML)

    Returns:
        Normalized text ready for risk pattern matching

    Examples:
        >>> normalize_for_risk_detection("<p>Hello  World</p>")
        'hello world'

        >>> normalize_for_risk_detection("URGENT\u200bMESSAGE")
        'urgentmessage'
    """
    if raw_content is None:
        return ""

    # Step 1: HTML to text
    text = html_to_text(raw_content)

    # Step 2: Unicode normalize (NFKC - compatibility decomposition + canonical composition)
    text = unicodedata.normalize("NFKC", text)

    # Step 3: Strip zero-width characters
    text = strip_zero_width_chars(text)

    # Step 4: Collapse whitespace
    text = collapse_whitespace(text)

    # Step 5: Lowercase
    text = text.lower()

    return text
