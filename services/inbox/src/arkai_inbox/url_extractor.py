"""URL extraction and phishing detection for email content.

Extracts URLs from HTML emails and performs security checks:
- Domain mismatch detection (phishing indicator)
- URL shortener identification
- Punycode/IDN normalization

Example:
    >>> html = '<a href="https://evil.com">Click paypal.com</a>'
    >>> urls = extract_urls_from_html(html)
    >>> urls[0].is_mismatch
    True
    >>> urls[0].href_domain
    'evil.com'
    >>> urls[0].text_domain
    'paypal.com'
"""

import re
from dataclasses import dataclass
from urllib.parse import urlparse

from bs4 import BeautifulSoup


@dataclass
class ExtractedUrl:
    """Represents a URL extracted from email content with security metadata."""

    href: str
    visible_text: str
    href_domain: str
    text_domain: str | None  # Domain mentioned in visible text, if any
    is_mismatch: bool  # text_domain != href_domain (phishing indicator)
    is_shortener: bool  # bit.ly, lnkd.in, t.co, etc.


# Known URL shortener domains
KNOWN_SHORTENERS = {
    "bit.ly",
    "lnkd.in",
    "t.co",
    "goo.gl",
    "tinyurl.com",
    "ow.ly",
    "buff.ly",
    "is.gd",
    "tiny.cc",
    "shorturl.at",
}

# Protocols to skip (not web URLs)
SKIP_PROTOCOLS = {"mailto:", "javascript:", "tel:", "sms:", "data:"}


def extract_urls_from_html(html_content: str) -> list[ExtractedUrl]:
    """Extract all <a href> links from HTML content.

    For each link:
    1. Get href attribute
    2. Get visible text
    3. Extract domain from href
    4. Check if visible text contains a different domain (phishing check)
    5. Check if domain is a known shortener

    Args:
        html_content: Raw HTML string from email body

    Returns:
        List of ExtractedUrl objects with security metadata
    """
    soup = BeautifulSoup(html_content, "lxml")
    extracted_urls = []

    for link in soup.find_all("a", href=True):
        href = link["href"].strip()

        # Skip non-web protocols
        if any(href.lower().startswith(proto) for proto in SKIP_PROTOCOLS):
            continue

        # Skip empty hrefs
        if not href or href == "#":
            continue

        # Extract visible text (combine all text nodes with spaces, strip whitespace)
        visible_text = link.get_text(separator=" ", strip=True)

        # Extract domain from href
        href_domain = extract_domain(href)
        if not href_domain:
            # Skip malformed URLs that don't parse
            continue

        # Check if visible text mentions a different domain
        text_domain = find_domain_in_text(visible_text)

        # Determine if this is a phishing mismatch
        is_mismatch = False
        if text_domain and text_domain != href_domain:
            is_mismatch = True

        # Check if domain is a known shortener
        is_shortener = href_domain in KNOWN_SHORTENERS

        extracted_urls.append(
            ExtractedUrl(
                href=href,
                visible_text=visible_text,
                href_domain=href_domain,
                text_domain=text_domain,
                is_mismatch=is_mismatch,
                is_shortener=is_shortener,
            )
        )

    return extracted_urls


def extract_domain(url: str) -> str:
    """Extract domain from URL, handling edge cases.

    Args:
        url: Full URL string

    Returns:
        Normalized domain (lowercase, www. stripped) or empty string if invalid
    """
    try:
        # Handle relative URLs by adding scheme if missing
        if not url.startswith(("http://", "https://", "//")):
            url = "http://" + url

        parsed = urlparse(url)
        domain = parsed.netloc.lower()

        # If no domain was parsed (malformed URL), return empty
        if not domain:
            return ""

        # Basic validation: domain should contain at least one dot
        # This filters out garbage like "not a url at all"
        if "." not in domain:
            return ""

        # Handle punycode (xn-- domains) - decode to Unicode then re-encode
        # This normalizes IDN homograph attacks
        if domain.startswith("xn--") or "xn--" in domain:
            try:
                # Decode punycode to unicode, then normalize back
                domain = domain.encode("ascii").decode("idna").lower()
            except (UnicodeError, UnicodeDecodeError):
                # If punycode decoding fails, keep as-is
                pass

        # Remove www. prefix for comparison
        if domain.startswith("www."):
            domain = domain[4:]

        return domain
    except Exception:
        # Return empty string for unparseable URLs
        return ""


def find_domain_in_text(text: str) -> str | None:
    """Find if text contains what looks like a domain.

    Catches phishing where visible text says "linkedin.com"
    but href goes to "linkedln-login.com" (note the 'l' vs 'i').

    Args:
        text: Visible link text to scan

    Returns:
        Domain found in text (normalized) or None
    """
    if not text:
        return None

    # Pattern: word characters, dot, 2-6 letter TLD
    # Matches: example.com, sub.example.co.uk, linkedin.com
    # Intentionally simple to catch obvious cases
    domain_pattern = r"\b([a-zA-Z0-9][-a-zA-Z0-9]*\.)+[a-zA-Z]{2,6}\b"

    match = re.search(domain_pattern, text)
    if match:
        found_domain = match.group(0).lower()
        # Normalize like we do for hrefs
        if found_domain.startswith("www."):
            found_domain = found_domain[4:]
        return found_domain

    return None
