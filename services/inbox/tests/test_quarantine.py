"""Tests for hard quarantine rules module."""

import pytest
from arkai_inbox.quarantine import (
    QuarantineResult,
    extract_email_address,
    evaluate_sender,
    is_approved_linkedin_domain,
    check_reply_to_mismatch,
    check_link_text_href_mismatch,
    evaluate_hard_quarantine,
    LINKEDIN_VALID_SENDERS,
)


class TestExtractEmailAddress:
    """Test email address extraction from various header formats."""

    def test_extract_with_angle_brackets(self):
        """Extract from format: 'Name <email@example.com>'."""
        result = extract_email_address('LinkedIn <notifications-noreply@linkedin.com>')
        assert result == 'notifications-noreply@linkedin.com'

    def test_extract_without_angle_brackets(self):
        """Extract from format: 'email@example.com'."""
        result = extract_email_address('notifications-noreply@linkedin.com')
        assert result == 'notifications-noreply@linkedin.com'

    def test_extract_with_mixed_case(self):
        """Ensure case normalization to lowercase."""
        result = extract_email_address('NOTIFICATIONS-NOREPLY@LINKEDIN.COM')
        assert result == 'notifications-noreply@linkedin.com'

    def test_extract_with_extra_whitespace(self):
        """Handle whitespace around email."""
        result = extract_email_address('  notifications-noreply@linkedin.com  ')
        assert result == 'notifications-noreply@linkedin.com'


class TestEvaluateSender:
    """Test 3-tier sender evaluation."""

    def test_tier1_pass_exact_match(self):
        """Known LinkedIn sender should PASS."""
        for sender in LINKEDIN_VALID_SENDERS:
            result = evaluate_sender(sender)
            assert result.tier == "PASS"
            assert result.reasons == []

    def test_tier1_pass_with_display_name(self):
        """Known LinkedIn sender with display name should PASS."""
        result = evaluate_sender('LinkedIn <notifications-noreply@linkedin.com>')
        assert result.tier == "PASS"
        assert result.reasons == []

    def test_tier2_review_unknown_linkedin_sender(self):
        """Unknown @linkedin.com sender should REVIEW."""
        result = evaluate_sender('unknown-sender@linkedin.com')
        assert result.tier == "REVIEW"
        assert "sender_not_in_exact_allowlist" in result.reasons

    def test_tier2_review_with_display_name(self):
        """Unknown @linkedin.com sender with display name should REVIEW."""
        result = evaluate_sender('LinkedIn <unknown@linkedin.com>')
        assert result.tier == "REVIEW"
        assert "sender_not_in_exact_allowlist" in result.reasons

    def test_tier3_quarantine_wrong_domain(self):
        """Sender from wrong domain should QUARANTINE."""
        test_cases = [
            'spoof@linkedln.com',  # Common typo: ln instead of in
            'phish@linkedin.net',
            'fake@linkedin.org',
            'evil@evil.com',
        ]
        for sender in test_cases:
            result = evaluate_sender(sender)
            assert result.tier == "QUARANTINE", f"Failed for {sender}"
            assert "sender_wrong_domain" in result.reasons


class TestIsApprovedLinkedInDomain:
    """Test LinkedIn domain validation."""

    def test_approved_domain_linkedin_com(self):
        """linkedin.com should be approved."""
        is_approved, warnings = is_approved_linkedin_domain('https://linkedin.com/jobs')
        assert is_approved is True
        assert warnings == []

    def test_approved_domain_www_linkedin_com(self):
        """www.linkedin.com should be approved."""
        is_approved, warnings = is_approved_linkedin_domain('https://www.linkedin.com/feed')
        assert is_approved is True
        assert warnings == []

    def test_shortener_domain_with_warning(self):
        """lnkd.in should be approved with warning."""
        is_approved, warnings = is_approved_linkedin_domain('https://lnkd.in/abc123')
        assert is_approved is True
        assert len(warnings) > 0
        assert any('shortener' in w for w in warnings)

    def test_unapproved_domain(self):
        """Non-LinkedIn domain should be rejected."""
        is_approved, warnings = is_approved_linkedin_domain('https://evil.com/phishing')
        assert is_approved is False
        assert len(warnings) > 0
        assert any('unapproved_domain' in w for w in warnings)

    def test_typo_domain(self):
        """Common typo domains should be rejected."""
        test_cases = [
            'https://linkedln.com',  # ln instead of in
            'https://linkedin.net',
            'https://linkedin.co',
        ]
        for url in test_cases:
            is_approved, warnings = is_approved_linkedin_domain(url)
            assert is_approved is False, f"Should reject {url}"


class TestCheckReplyToMismatch:
    """Test Reply-To header mismatch detection."""

    def test_no_reply_to_header(self):
        """No Reply-To header should not trigger mismatch."""
        result = check_reply_to_mismatch(
            'notifications-noreply@linkedin.com',
            None
        )
        assert result is False

    def test_reply_to_matches_from(self):
        """Matching Reply-To should not trigger mismatch."""
        result = check_reply_to_mismatch(
            'notifications-noreply@linkedin.com',
            'notifications-noreply@linkedin.com'
        )
        assert result is False

    def test_reply_to_differs_from_from(self):
        """Mismatched Reply-To should trigger mismatch."""
        result = check_reply_to_mismatch(
            'notifications-noreply@linkedin.com',
            'phishing@evil.com'
        )
        assert result is True

    def test_reply_to_mismatch_with_display_names(self):
        """Mismatch detection with display names."""
        result = check_reply_to_mismatch(
            'LinkedIn <notifications-noreply@linkedin.com>',
            'Evil <phishing@evil.com>'
        )
        assert result is True


class TestCheckLinkTextHrefMismatch:
    """Test link text/href mismatch detection (phishing indicator)."""

    def test_no_domain_in_text(self):
        """Link text without domain should not trigger mismatch."""
        link = {'href': 'https://linkedin.com/jobs', 'text': 'View Job'}
        result = check_link_text_href_mismatch(link)
        assert result is False

    def test_matching_domain_in_text_and_href(self):
        """Matching domains should not trigger mismatch."""
        link = {'href': 'https://linkedin.com/jobs', 'text': 'linkedin.com/jobs'}
        result = check_link_text_href_mismatch(link)
        assert result is False

    def test_mismatched_domain_phishing(self):
        """Text showing linkedin.com but href to evil.com should trigger."""
        link = {'href': 'https://evil.com/phishing', 'text': 'linkedin.com/jobs'}
        result = check_link_text_href_mismatch(link)
        assert result is True

    def test_www_prefix_normalization(self):
        """www. prefix should be normalized for comparison."""
        link = {'href': 'https://www.linkedin.com/jobs', 'text': 'linkedin.com'}
        result = check_link_text_href_mismatch(link)
        assert result is False


class TestEvaluateHardQuarantine:
    """Test combined hard quarantine evaluation."""

    def test_legitimate_linkedin_email(self):
        """Legitimate LinkedIn email should PASS all rules."""
        email_data = {
            'from': 'LinkedIn <notifications-noreply@linkedin.com>',
            'reply_to': None,
            'html_body': '<a href="https://linkedin.com/jobs">View Jobs</a>'
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "PASS"
        assert result.reasons == []

    def test_sender_wrong_domain_quarantine(self):
        """Wrong sender domain should QUARANTINE."""
        email_data = {
            'from': 'phishing@evil.com',
            'reply_to': None,
            'html_body': None
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "QUARANTINE"
        assert "sender_wrong_domain" in result.reasons

    def test_unknown_linkedin_sender_review(self):
        """Unknown @linkedin.com sender should REVIEW."""
        email_data = {
            'from': 'unknown-sender@linkedin.com',
            'reply_to': None,
            'html_body': None
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "REVIEW"
        assert "sender_not_in_exact_allowlist" in result.reasons

    def test_reply_to_mismatch_quarantine(self):
        """Reply-To mismatch should QUARANTINE."""
        email_data = {
            'from': 'notifications-noreply@linkedin.com',
            'reply_to': 'phishing@evil.com',
            'html_body': None
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "QUARANTINE"
        assert "reply_to_mismatch" in result.reasons

    def test_deep_link_wrong_domain_quarantine(self):
        """Link to non-LinkedIn domain should QUARANTINE."""
        email_data = {
            'from': 'notifications-noreply@linkedin.com',
            'reply_to': None,
            'html_body': '<a href="https://evil.com/phishing">Click Here</a>'
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "QUARANTINE"
        assert "deep_link_wrong_domain" in result.reasons

    def test_link_text_href_mismatch_quarantine(self):
        """Link text/href mismatch should QUARANTINE."""
        email_data = {
            'from': 'notifications-noreply@linkedin.com',
            'reply_to': None,
            'html_body': '<a href="https://evil.com">linkedin.com/jobs</a>'
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "QUARANTINE"
        assert "link_text_href_mismatch" in result.reasons

    def test_multiple_violations(self):
        """Multiple violations should all be reported."""
        email_data = {
            'from': 'phishing@evil.com',
            'reply_to': 'another-evil@evil.com',
            'html_body': '<a href="https://evil.com">linkedin.com</a>'
        }
        result = evaluate_hard_quarantine(email_data)
        assert result.tier == "QUARANTINE"
        # Should have multiple reasons
        assert len(result.reasons) > 1
        assert "sender_wrong_domain" in result.reasons

    def test_no_duplicate_reasons(self):
        """Duplicate reasons should be removed."""
        email_data = {
            'from': 'notifications-noreply@linkedin.com',
            'reply_to': None,
            'html_body': '''
                <a href="https://evil.com">link1</a>
                <a href="https://evil.com">link2</a>
            '''
        }
        result = evaluate_hard_quarantine(email_data)
        # Should only have one instance of deep_link_wrong_domain
        assert result.reasons.count("deep_link_wrong_domain") == 1


class TestSpoofedDomains:
    """Test detection of common LinkedIn domain spoofs."""

    def test_linkedln_com_typo(self):
        """linkedln.com (ln instead of in) should QUARANTINE."""
        result = evaluate_sender('notifications@linkedln.com')
        assert result.tier == "QUARANTINE"
        assert "sender_wrong_domain" in result.reasons

    def test_linkedin_net_spoof(self):
        """linkedin.net should QUARANTINE."""
        result = evaluate_sender('notifications@linkedin.net')
        assert result.tier == "QUARANTINE"
        assert "sender_wrong_domain" in result.reasons

    def test_linkedin_org_spoof(self):
        """linkedin.org should QUARANTINE."""
        result = evaluate_sender('notifications@linkedin.org')
        assert result.tier == "QUARANTINE"
        assert "sender_wrong_domain" in result.reasons

    def test_linked1n_com_number_spoof(self):
        """linked1n.com (1 instead of i) should QUARANTINE."""
        result = evaluate_sender('notifications@linked1n.com')
        assert result.tier == "QUARANTINE"
        assert "sender_wrong_domain" in result.reasons
