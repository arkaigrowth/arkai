# RALPH Constraints

> These are the psychological safety rails and hard rules for all sessions.
> Edit sparingly. These persist across all sessions.

---

## Hard Rules

1. **Never commit directly to main/master** - Always use feature branches
2. **Never auto-push to remote** - Require explicit user confirmation
3. **Never delete user data** - Archive, don't destroy
4. **Never store secrets in artifacts** - Scrub API keys, tokens, passwords

---

## Session Hygiene

1. **Read before write** - Always understand existing code before modifying
2. **Test before commit** - Run tests after changes, before committing
3. **Document decisions** - Every non-trivial choice needs rationale
4. **Clean up temp files** - Remove debugging artifacts before session end

---

## Quality Standards

1. **No TODO comments for core functionality** - Finish what you start
2. **No placeholder implementations** - Real code or nothing
3. **No enterprise bloat** - Build what's asked, not a framework

---

## Project-Specific

<!-- Add project-specific constraints below -->

- arkai is a Rust project - use `cargo check` and `cargo test`
- Output files go in canonical locations (see CLAUDE.md)
- Evidence claims require source verification

---

## Anti-Patterns to Avoid

- **Sycophantic behavior**: No "great question!" or excessive praise
- **Marketing language**: No "blazingly fast" or "revolutionary"
- **Fake metrics**: No invented percentages or time estimates
- **Scope creep**: Build what's asked, not what might be useful later

---

*Last updated: 2026-01-18*
