"""
Tests for Pre-Gate normalization module.
"""

import pytest

from arkai_inbox.normalize import (
    collapse_whitespace,
    extract_first_n_chars,
    extract_last_n_chars,
    html_to_text,
    normalize_for_risk_detection,
    strip_zero_width_chars,
)


class TestHtmlToText:
    """Tests for HTML to text conversion."""

    def test_basic_html_stripping(self):
        """Test basic HTML tag removal."""
        html = "<p>Hello</p>"
        result = html_to_text(html)
        assert result == "Hello"

    def test_nested_tags(self):
        """Test nested HTML tags."""
        html = "<div><p>Hello <strong>World</strong></p></div>"
        result = html_to_text(html)
        assert "Hello" in result
        assert "World" in result

    def test_script_and_style_removal(self):
        """Test that script and style tags are removed."""
        html = """
        <html>
            <head><style>body { color: red; }</style></head>
            <body>
                <p>Content</p>
                <script>alert('test');</script>
            </body>
        </html>
        """
        result = html_to_text(html)
        assert "Content" in result
        assert "color: red" not in result
        assert "alert" not in result

    def test_empty_string(self):
        """Test empty string handling."""
        assert html_to_text("") == ""

    def test_plain_text(self):
        """Test plain text (no HTML)."""
        text = "Just plain text"
        result = html_to_text(text)
        assert "Just plain text" in result


class TestStripZeroWidthChars:
    """Tests for zero-width character removal."""

    def test_zero_width_space(self):
        """Test removal of zero-width space."""
        text = "Hello\u200bWorld"
        result = strip_zero_width_chars(text)
        assert result == "HelloWorld"

    def test_zero_width_non_joiner(self):
        """Test removal of zero-width non-joiner."""
        text = "Hello\u200cWorld"
        result = strip_zero_width_chars(text)
        assert result == "HelloWorld"

    def test_zero_width_joiner(self):
        """Test removal of zero-width joiner."""
        text = "Hello\u200dWorld"
        result = strip_zero_width_chars(text)
        assert result == "HelloWorld"

    def test_bom(self):
        """Test removal of zero-width no-break space (BOM)."""
        text = "\ufeffHello World"
        result = strip_zero_width_chars(text)
        assert result == "Hello World"

    def test_soft_hyphen(self):
        """Test removal of soft hyphen."""
        text = "Hello\u00adWorld"
        result = strip_zero_width_chars(text)
        assert result == "HelloWorld"

    def test_multiple_zero_width_chars(self):
        """Test removal of multiple zero-width characters."""
        text = "\ufeffHello\u200b\u200cWorld\u200d\u00ad"
        result = strip_zero_width_chars(text)
        assert result == "HelloWorld"

    def test_empty_string(self):
        """Test empty string handling."""
        assert strip_zero_width_chars("") == ""


class TestCollapseWhitespace:
    """Tests for whitespace normalization."""

    def test_multiple_spaces(self):
        """Test collapsing multiple spaces."""
        text = "Hello    World"
        result = collapse_whitespace(text)
        assert result == "Hello World"

    def test_newlines(self):
        """Test collapsing newlines."""
        text = "Hello\n\nWorld"
        result = collapse_whitespace(text)
        assert result == "Hello World"

    def test_tabs(self):
        """Test collapsing tabs."""
        text = "Hello\t\tWorld"
        result = collapse_whitespace(text)
        assert result == "Hello World"

    def test_mixed_whitespace(self):
        """Test collapsing mixed whitespace characters."""
        text = "Hello \t\n  World"
        result = collapse_whitespace(text)
        assert result == "Hello World"

    def test_leading_trailing_whitespace(self):
        """Test stripping leading/trailing whitespace."""
        text = "  Hello World  "
        result = collapse_whitespace(text)
        assert result == "Hello World"

    def test_only_whitespace(self):
        """Test string with only whitespace."""
        text = "   \t\n  "
        result = collapse_whitespace(text)
        assert result == ""


class TestExtractFirstNChars:
    """Tests for first N characters extraction."""

    def test_extract_first_200_chars(self):
        """Test extracting first 200 characters."""
        text = "a" * 500
        result = extract_first_n_chars(text, 200)
        assert len(result) == 200
        assert result == "a" * 200

    def test_text_shorter_than_n(self):
        """Test text shorter than requested length."""
        text = "Hello"
        result = extract_first_n_chars(text, 200)
        assert result == "Hello"

    def test_custom_n(self):
        """Test custom N value."""
        text = "Hello World"
        result = extract_first_n_chars(text, 5)
        assert result == "Hello"

    def test_empty_string(self):
        """Test empty string."""
        result = extract_first_n_chars("", 200)
        assert result == ""

    def test_default_n(self):
        """Test default N=200."""
        text = "a" * 500
        result = extract_first_n_chars(text)
        assert len(result) == 200


class TestExtractLastNChars:
    """Tests for last N characters extraction."""

    def test_extract_last_200_chars(self):
        """Test extracting last 200 characters."""
        text = "a" * 500
        result = extract_last_n_chars(text, 200)
        assert len(result) == 200
        assert result == "a" * 200

    def test_text_shorter_than_n(self):
        """Test text shorter than requested length."""
        text = "Hello"
        result = extract_last_n_chars(text, 200)
        assert result == "Hello"

    def test_custom_n(self):
        """Test custom N value."""
        text = "Hello World"
        result = extract_last_n_chars(text, 5)
        assert result == "World"

    def test_empty_string(self):
        """Test empty string."""
        result = extract_last_n_chars("", 200)
        assert result == ""

    def test_default_n(self):
        """Test default N=200."""
        text = "a" * 500
        result = extract_last_n_chars(text)
        assert len(result) == 200


class TestNormalizeForRiskDetection:
    """Tests for complete normalization pipeline."""

    def test_basic_html_normalization(self):
        """Test basic HTML to normalized text."""
        html = "<p>Hello</p>"
        result = normalize_for_risk_detection(html)
        assert result == "hello"

    def test_unicode_normalization(self):
        """Test Unicode normalization (full-width to ASCII)."""
        # Full-width "Hello" should normalize to regular ASCII
        text = "ＨＥＬＬＯ"
        result = normalize_for_risk_detection(text)
        assert result == "hello"

    def test_zero_width_char_removal(self):
        """Test zero-width character removal in pipeline."""
        text = "URGENT\u200bMESSAGE"
        result = normalize_for_risk_detection(text)
        assert result == "urgentmessage"

    def test_whitespace_collapse(self):
        """Test whitespace collapsing in pipeline."""
        text = "Hello    World\n\nTest"
        result = normalize_for_risk_detection(text)
        assert result == "hello world test"

    def test_combined_messy_email(self):
        """Test complete pipeline with messy HTML email."""
        html = """
        <html>
            <body>
                <p>URGENT\u200b  MESSAGE</p>
                <p>Click   <a href="#">HERE</a></p>
                <script>alert('phishing');</script>
            </body>
        </html>
        """
        result = normalize_for_risk_detection(html)

        # Should be normalized, lowercased, whitespace collapsed
        assert "urgent" in result
        assert "message" in result
        assert "click" in result
        assert "here" in result

        # Should NOT contain script content
        assert "alert" not in result
        assert "phishing" not in result

        # Should have collapsed whitespace
        assert "  " not in result

    def test_none_input(self):
        """Test None input handling."""
        result = normalize_for_risk_detection(None)
        assert result == ""

    def test_empty_string(self):
        """Test empty string handling."""
        result = normalize_for_risk_detection("")
        assert result == ""

    def test_whitespace_only(self):
        """Test whitespace-only input."""
        result = normalize_for_risk_detection("   \t\n  ")
        assert result == ""

    def test_unicode_with_html(self):
        """Test Unicode normalization with HTML."""
        html = "<p>ＨＥＬＬＯ  World</p>"
        result = normalize_for_risk_detection(html)
        assert result == "hello world"

    def test_multiple_zero_width_in_html(self):
        """Test multiple zero-width chars in HTML."""
        html = "<p>UR\u200bGE\u200cNT\u200d</p>"
        result = normalize_for_risk_detection(html)
        assert result == "urgent"

    def test_realistic_phishing_email(self):
        """Test realistic phishing email normalization."""
        html = """
        <html>
        <body>
            <p><strong>URGENT\u200b SECURITY ALERT</strong></p>
            <p>Your   account   will be   suspended.</p>
            <p>Click <a href="http://evil.com">here</a> IMMEDIATELY.</p>
        </body>
        </html>
        """
        result = normalize_for_risk_detection(html)

        # All key phishing terms should be present and normalized
        assert "urgent" in result
        assert "security" in result
        assert "alert" in result
        assert "account" in result
        assert "suspended" in result
        assert "click" in result
        assert "here" in result
        assert "immediately" in result

        # Should be single-spaced
        assert "  " not in result

        # Should be lowercase
        assert result == result.lower()
