# Agent Instructions (Canonical)

This file is the single source of truth for agent behavior in this repo.
Use it for Codex and Claude Code sessions.

## 0) Session Start
- Read `/Users/alexkamysz/AI/arkai/docs/CONTEXT.md`, `/Users/alexkamysz/AI/arkai/docs/THREADS.md`, and `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md` before doing work.
- Scan `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/` for recent entries.
- Do not edit `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md` unless you are the current `Owner` in that file.
- If you are not the owner, write updates to a dated handoff in `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/`.

## 1) Non-Negotiables
- Do not invent APIs, configs, tools, or commands; verify from repo files first.
- Keep changes small and reversible.
- Never print, store, or commit secrets.
- Prefer read-only scans before edits on risky work.

## 2) Cross-Repo Governance
- Canonical authority: See `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md`.
- Do not duplicate authority matrices elsewhere. Other files should include a one-line pointer only.

## 3) AI Coordination (`docs/ai/`)
- Protocol and templates: `/Users/alexkamysz/AI/arkai/docs/ai/README.md`
- Shared fast snapshot (single writer): `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md`
- Handoffs: `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/YYYY-MM-DD-tool-summary.md`

## 4) Verification
- Run `/Users/alexkamysz/AI/arkai/scripts/verify_ai_docs.sh` for docs/ai checks.
- Pre-commit hook enforces governance automatically when `docs/ai/` files are staged.
- Setup (one-time per clone): `git config core.hooksPath .githooks`
