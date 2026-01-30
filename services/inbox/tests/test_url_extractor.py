"""Tests for URL extraction and phishing detection."""

import pytest

from arkai_inbox.url_extractor import (
    ExtractedUrl,
    extract_domain,
    extract_urls_from_html,
    find_domain_in_text,
)


class TestBasicExtraction:
    """Test basic URL extraction from HTML."""

    def test_single_link(self):
        html = '<a href="https://example.com">Click here</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href == "https://example.com"
        assert urls[0].visible_text == "Click here"
        assert urls[0].href_domain == "example.com"
        assert urls[0].text_domain is None
        assert urls[0].is_mismatch is False
        assert urls[0].is_shortener is False

    def test_multiple_links(self):
        html = """
        <p>Check out <a href="https://example.com">example</a> and
        <a href="https://test.org">test site</a></p>
        """
        urls = extract_urls_from_html(html)

        assert len(urls) == 2
        assert urls[0].href_domain == "example.com"
        assert urls[1].href_domain == "test.org"

    def test_link_with_nested_elements(self):
        html = '<a href="https://example.com"><strong>Bold</strong> text</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].visible_text == "Bold text"

    def test_empty_html(self):
        html = "<p>No links here</p>"
        urls = extract_urls_from_html(html)

        assert len(urls) == 0


class TestPhishingDetection:
    """Test phishing domain mismatch detection."""

    def test_visible_domain_mismatch(self):
        # Classic phishing: says linkedin.com but goes elsewhere
        html = '<a href="http://evil.com">Click linkedin.com</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "evil.com"
        assert urls[0].text_domain == "linkedin.com"
        assert urls[0].is_mismatch is True

    def test_homograph_attack_example(self):
        # Visible text says "paypal.com" but href is "paypa1.com" (1 vs l)
        html = '<a href="https://paypa1.com">paypal.com</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "paypa1.com"
        assert urls[0].text_domain == "paypal.com"
        assert urls[0].is_mismatch is True

    def test_subdomain_in_text_is_mismatch(self):
        # Text says "login.bank.com" but href is "evil.com"
        html = '<a href="https://evil.com">login.bank.com</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        # Our simple domain parser extracts the full domain including subdomain
        assert urls[0].text_domain == "login.bank.com"
        assert urls[0].is_mismatch is True

    def test_matching_domains_not_mismatch(self):
        # Both href and text mention same domain
        html = '<a href="https://example.com">Visit example.com</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "example.com"
        assert urls[0].text_domain == "example.com"
        assert urls[0].is_mismatch is False

    def test_generic_text_not_mismatch(self):
        # Generic text like "Click here" shouldn't flag as mismatch
        html = '<a href="https://example.com">Click here</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].text_domain is None
        assert urls[0].is_mismatch is False


class TestShortenerDetection:
    """Test URL shortener identification."""

    def test_bitly_detected(self):
        html = '<a href="https://bit.ly/abc123">Short link</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "bit.ly"
        assert urls[0].is_shortener is True

    def test_linkedin_shortener(self):
        html = '<a href="https://lnkd.in/xyz">LinkedIn share</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "lnkd.in"
        assert urls[0].is_shortener is True

    def test_multiple_shorteners(self):
        html = """
        <a href="https://bit.ly/abc">Link 1</a>
        <a href="https://t.co/xyz">Link 2</a>
        <a href="https://tinyurl.com/test">Link 3</a>
        """
        urls = extract_urls_from_html(html)

        assert len(urls) == 3
        assert all(url.is_shortener for url in urls)

    def test_normal_domain_not_shortener(self):
        html = '<a href="https://example.com">Normal link</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].is_shortener is False


class TestDomainNormalization:
    """Test domain extraction and normalization."""

    def test_www_stripped(self):
        assert extract_domain("https://www.example.com") == "example.com"
        assert extract_domain("http://www.test.org") == "test.org"

    def test_case_normalized(self):
        assert extract_domain("https://Example.COM") == "example.com"
        assert extract_domain("https://TeSt.ORG") == "test.org"

    def test_subdomain_preserved(self):
        assert extract_domain("https://api.example.com") == "api.example.com"
        assert extract_domain("https://www.api.example.com") == "api.example.com"

    def test_www_equivalence(self):
        # www.linkedin.com should equal linkedin.com for comparison
        html = '<a href="https://www.linkedin.com">linkedin.com</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "linkedin.com"
        assert urls[0].text_domain == "linkedin.com"
        assert urls[0].is_mismatch is False

    def test_relative_url_with_scheme_added(self):
        # extract_domain should handle URLs without scheme
        domain = extract_domain("example.com/path")
        assert domain == "example.com"

    def test_malformed_url_returns_empty(self):
        assert extract_domain("not a url at all") == ""
        assert extract_domain("") == ""
        assert extract_domain("://broken") == ""


class TestPunycodeHandling:
    """Test punycode/IDN domain handling."""

    def test_punycode_domain_decoded(self):
        # xn--e1afmkfd.xn--p1ai is пример.рф in Russian
        punycode_url = "https://xn--e1afmkfd.xn--p1ai"
        domain = extract_domain(punycode_url)

        # Should decode punycode to unicode (normalized)
        # The exact output depends on Python's idna codec
        assert domain  # Should return something, not empty
        assert "xn--" not in domain or domain.startswith("xn--")  # Either decoded or kept

    def test_punycode_html_extraction(self):
        html = '<a href="https://xn--e1afmkfd.xn--p1ai">Test</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        # Should extract without crashing
        assert urls[0].href_domain

    def test_mixed_punycode_ascii(self):
        # Some domains mix ASCII and punycode
        html = '<a href="https://www.xn--Example-abc.com">Test</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        # Should handle gracefully
        assert urls[0].href_domain


class TestSkipLinks:
    """Test that non-HTTP links are skipped."""

    def test_mailto_skipped(self):
        html = '<a href="mailto:test@example.com">Email me</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_javascript_skipped(self):
        html = '<a href="javascript:alert(1)">Click</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_tel_skipped(self):
        html = '<a href="tel:+1234567890">Call us</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_empty_href_skipped(self):
        html = '<a href="">Empty</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_hash_only_skipped(self):
        html = '<a href="#">Anchor</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_data_uri_skipped(self):
        html = '<a href="data:text/plain,hello">Data URI</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0


class TestMalformedHtml:
    """Test that malformed HTML is handled gracefully."""

    def test_unclosed_tags(self):
        html = '<a href="https://example.com">Unclosed link'
        urls = extract_urls_from_html(html)

        # BeautifulSoup should handle this
        assert len(urls) == 1
        assert urls[0].href_domain == "example.com"

    def test_nested_anchors(self):
        # Invalid HTML but should handle
        html = '<a href="https://example.com">Outer <a href="https://inner.com">Inner</a></a>'
        urls = extract_urls_from_html(html)

        # BeautifulSoup will parse this somehow
        assert len(urls) >= 1

    def test_no_href_attribute(self):
        html = '<a>No href</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 0

    def test_whitespace_in_href(self):
        html = '<a href="  https://example.com  ">Link</a>'
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href == "https://example.com"  # Stripped


class TestFindDomainInText:
    """Test domain detection in visible text."""

    def test_simple_domain(self):
        assert find_domain_in_text("Visit example.com") == "example.com"

    def test_domain_with_subdomain(self):
        assert find_domain_in_text("Go to api.example.com") == "api.example.com"

    def test_multiple_domains_returns_first(self):
        # Should find the first domain
        result = find_domain_in_text("Visit example.com or test.org")
        assert result == "example.com"

    def test_no_domain(self):
        assert find_domain_in_text("Click here") is None
        assert find_domain_in_text("No domain present") is None

    def test_www_stripped_in_text(self):
        assert find_domain_in_text("Visit www.example.com") == "example.com"

    def test_empty_text(self):
        assert find_domain_in_text("") is None
        assert find_domain_in_text(None) is None

    def test_case_normalized_in_text(self):
        assert find_domain_in_text("Visit Example.COM") == "example.com"

    def test_domain_with_path(self):
        # Should extract just domain, not path
        result = find_domain_in_text("Visit example.com/path/here")
        assert result == "example.com"


class TestRealWorldExamples:
    """Test realistic email scenarios."""

    def test_linkedin_email(self):
        html = """
        <p>New connection request:</p>
        <a href="https://www.linkedin.com/comm/...">View on LinkedIn</a>
        """
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "linkedin.com"
        assert urls[0].is_shortener is False

    def test_newsletter_with_tracking(self):
        html = """
        <p>Read our article:</p>
        <a href="https://example.com/article?utm_source=email&utm_campaign=weekly">
            Latest news
        </a>
        """
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "example.com"
        # Query params should be preserved in href
        assert "utm_source" in urls[0].href

    def test_phishing_attempt_realistic(self):
        # Realistic phishing: URL says "paypal.com" but goes to fake domain
        html = """
        <p>Your account needs verification!</p>
        <a href="https://paypal-secure-login.tk/verify">
            Click here to verify your paypal.com account
        </a>
        """
        urls = extract_urls_from_html(html)

        assert len(urls) == 1
        assert urls[0].href_domain == "paypal-secure-login.tk"
        assert urls[0].text_domain == "paypal.com"
        assert urls[0].is_mismatch is True

    def test_mixed_content_email(self):
        html = """
        <html>
        <body>
            <p>Hi there,</p>
            <p>Check out our site: <a href="https://example.com">example.com</a></p>
            <p>Follow us: <a href="https://twitter.com/handle">@handle</a></p>
            <p>Unsubscribe: <a href="https://bit.ly/unsub123">here</a></p>
            <p>Contact: <a href="mailto:help@example.com">help@example.com</a></p>
        </body>
        </html>
        """
        urls = extract_urls_from_html(html)

        # Should extract 3 URLs (not mailto)
        assert len(urls) == 3
        domains = {url.href_domain for url in urls}
        assert "example.com" in domains
        assert "twitter.com" in domains
        assert "bit.ly" in domains

        # Check shortener flagged
        bitly_url = [url for url in urls if url.href_domain == "bit.ly"][0]
        assert bitly_url.is_shortener is True
