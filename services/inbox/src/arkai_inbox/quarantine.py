"""Hard quarantine rules for inbox security.

This module implements non-negotiable security rules that override any scoring system.
A single hard quarantine violation sends the email directly to quarantine for manual review.
"""

from dataclasses import dataclass
from typing import Literal
import re
from urllib.parse import urlparse


@dataclass
class QuarantineResult:
    """Result of quarantine evaluation.

    Attributes:
        tier: PASS (safe), REVIEW (suspicious), or QUARANTINE (blocked)
        reasons: List of rule violations that triggered this tier
    """
    tier: Literal["PASS", "REVIEW", "QUARANTINE"]
    reasons: list[str]


# LinkedIn valid senders (exact match = PASS)
LINKEDIN_VALID_SENDERS = [
    "notifications-noreply@linkedin.com",
    "messages-noreply@linkedin.com",
    "invitations@linkedin.com",
    "jobs-noreply@linkedin.com",
    "jobalerts-noreply@linkedin.com",  # Job alert emails
]

# LinkedIn approved link domains
LINKEDIN_APPROVED_DOMAINS = {"linkedin.com", "www.linkedin.com"}
LINKEDIN_SUSPICIOUS_DOMAINS = {"lnkd.in"}  # Shortener - allowed with warning

# Expected third-party domains in LinkedIn email footers (app stores, etc.)
EXPECTED_THIRD_PARTY_DOMAINS = {
    "play.google.com",      # Google Play Store
    "itunes.apple.com",     # iOS App Store (legacy)
    "apps.apple.com",       # iOS App Store (new)
    "apps.microsoft.com",   # Microsoft Store
    "support.apple.com",    # Apple support links
    "support.google.com",   # Google support links
}


def extract_email_address(from_header: str) -> str:
    """Extract email from From header.

    Handles both formats:
    - 'LinkedIn <notifications-noreply@linkedin.com>' -> 'notifications-noreply@linkedin.com'
    - 'notifications-noreply@linkedin.com' -> 'notifications-noreply@linkedin.com'

    Args:
        from_header: Raw From header value

    Returns:
        Email address in lowercase
    """
    match = re.search(r'<([^>]+)>', from_header)
    if match:
        return match.group(1).lower()
    return from_header.strip().lower()


def evaluate_sender(from_header: str) -> QuarantineResult:
    """3-tier sender evaluation.

    Tiers:
    - PASS: Exact match in LINKEDIN_VALID_SENDERS
    - REVIEW: @linkedin.com domain but unknown sender
    - QUARANTINE: Wrong domain entirely

    Args:
        from_header: Raw From header value

    Returns:
        QuarantineResult with tier and reasons
    """
    sender = extract_email_address(from_header)

    # Tier 1: PASS (exact match)
    if sender in LINKEDIN_VALID_SENDERS:
        return QuarantineResult(tier="PASS", reasons=[])

    # Tier 2: REVIEW (linkedin.com domain but unknown sender)
    if sender.endswith("@linkedin.com"):
        return QuarantineResult(tier="REVIEW", reasons=["sender_not_in_exact_allowlist"])

    # Tier 3: QUARANTINE (wrong domain)
    return QuarantineResult(tier="QUARANTINE", reasons=["sender_wrong_domain"])


def is_approved_linkedin_domain(url: str) -> tuple[bool, list[str]]:
    """Check if URL domain is approved for LinkedIn links.

    Args:
        url: URL to check (can be partial or full)

    Returns:
        (is_approved, warnings) where is_approved is True if domain is safe,
        and warnings contains any advisory messages
    """
    warnings = []

    # Handle URLs without scheme
    if not url.startswith(('http://', 'https://')):
        url = 'https://' + url

    try:
        parsed = urlparse(url)
        domain = parsed.netloc.lower()

        # Remove port if present
        if ':' in domain:
            domain = domain.split(':')[0]

        # Check approved LinkedIn domains
        if domain in LINKEDIN_APPROVED_DOMAINS:
            return (True, warnings)

        # Check suspicious but allowed domains (shorteners)
        if domain in LINKEDIN_SUSPICIOUS_DOMAINS:
            warnings.append(f"shortener_domain_{domain}")
            return (True, warnings)

        # Check expected third-party domains (app stores, etc.)
        if domain in EXPECTED_THIRD_PARTY_DOMAINS:
            return (True, warnings)

        # Not approved
        return (False, [f"unapproved_domain_{domain}"])

    except Exception:
        # Malformed URL
        return (False, ["malformed_url"])


def check_reply_to_mismatch(from_header: str, reply_to: str | None) -> bool:
    """Check if Reply-To differs from From address.

    Args:
        from_header: Raw From header value
        reply_to: Raw Reply-To header value or None

    Returns:
        True if there's a mismatch (quarantine), False otherwise
    """
    if reply_to is None:
        return False

    from_addr = extract_email_address(from_header)
    reply_to_addr = extract_email_address(reply_to)

    return from_addr != reply_to_addr


def extract_links_from_html(html_body: str) -> list[dict[str, str]]:
    """Extract links from HTML body with their visible text.

    Args:
        html_body: HTML content

    Returns:
        List of dicts with 'href' and 'text' keys
    """
    try:
        from bs4 import BeautifulSoup
    except ImportError:
        # Fallback: simple regex extraction
        return _extract_links_regex(html_body)

    soup = BeautifulSoup(html_body, 'lxml')
    links = []

    for anchor in soup.find_all('a', href=True):
        links.append({
            'href': anchor['href'],
            'text': anchor.get_text(strip=True)
        })

    return links


def _extract_links_regex(html_body: str) -> list[dict[str, str]]:
    """Fallback link extraction using regex."""
    links = []
    # Simple pattern: <a href="...">text</a>
    pattern = r'<a[^>]+href=["\']([^"\']+)["\'][^>]*>(.*?)</a>'

    for match in re.finditer(pattern, html_body, re.IGNORECASE | re.DOTALL):
        href = match.group(1)
        text = re.sub(r'<[^>]+>', '', match.group(2)).strip()
        links.append({'href': href, 'text': text})

    return links


def check_link_text_href_mismatch(link: dict[str, str]) -> bool:
    """Check if visible link text domain differs from actual href domain.

    This is a common phishing indicator:
    <a href="evil.com">linkedin.com/jobs</a>

    Args:
        link: Dict with 'href' and 'text' keys

    Returns:
        True if there's a mismatch (quarantine), False otherwise
    """
    href = link.get('href', '')
    text = link.get('text', '')

    # Extract domain from href
    if not href.startswith(('http://', 'https://')):
        href = 'https://' + href

    try:
        href_domain = urlparse(href).netloc.lower()
        if ':' in href_domain:
            href_domain = href_domain.split(':')[0]
    except Exception:
        return True  # Malformed URL is suspicious

    # Check if text contains a domain that differs from href
    # Look for patterns like "linkedin.com", "www.linkedin.com" in text
    domain_pattern = r'(?:https?://)?(?:www\.)?([a-z0-9-]+\.[a-z]{2,})'
    text_domains = re.findall(domain_pattern, text.lower())

    if not text_domains:
        return False  # No domain in text, not a mismatch

    # If text contains a domain, it should match href domain
    for text_domain in text_domains:
        # Normalize: remove www. prefix for comparison
        text_domain_normalized = text_domain.replace('www.', '')
        href_domain_normalized = href_domain.replace('www.', '')

        if text_domain_normalized != href_domain_normalized:
            return True  # Mismatch detected

    return False


def evaluate_hard_quarantine(email_data: dict) -> QuarantineResult:
    """Run all hard quarantine rules.

    Args:
        email_data: Dict with keys:
            - from: From header
            - reply_to: Reply-To header (optional)
            - html_body: HTML body content (optional)

    Returns:
        QuarantineResult with highest severity tier and all violations
    """
    reasons = []
    highest_tier = "PASS"

    # Rule 1: sender_wrong_domain
    sender_result = evaluate_sender(email_data.get('from', ''))
    if sender_result.tier == "QUARANTINE":
        reasons.extend(sender_result.reasons)
        highest_tier = "QUARANTINE"
    elif sender_result.tier == "REVIEW" and highest_tier == "PASS":
        reasons.extend(sender_result.reasons)
        highest_tier = "REVIEW"

    # Rule 2: reply_to_mismatch
    if check_reply_to_mismatch(
        email_data.get('from', ''),
        email_data.get('reply_to')
    ):
        reasons.append("reply_to_mismatch")
        highest_tier = "QUARANTINE"

    # Rule 3 & 4: link analysis (only if html_body provided)
    html_body = email_data.get('html_body')
    if html_body:
        links = extract_links_from_html(html_body)

        for link in links:
            href = link.get('href', '')

            # Rule 3: deep_link_wrong_domain
            # Check main CTAs (we'll consider all links for now)
            is_approved, warnings = is_approved_linkedin_domain(href)
            if not is_approved:
                reasons.append("deep_link_wrong_domain")
                highest_tier = "QUARANTINE"

            # Rule 4: link_text_href_mismatch
            if check_link_text_href_mismatch(link):
                reasons.append("link_text_href_mismatch")
                highest_tier = "QUARANTINE"

    # Remove duplicates while preserving order
    unique_reasons = []
    seen = set()
    for reason in reasons:
        if reason not in seen:
            unique_reasons.append(reason)
            seen.add(reason)

    return QuarantineResult(tier=highest_tier, reasons=unique_reasons)
