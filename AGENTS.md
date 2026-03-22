# Agent Instructions (Canonical)

This file is the single source of truth for agent behavior in this repo.
Use it for Codex and Claude Code sessions.

## 0) Session Start
- Read `~/AI/arkai/docs/CONTEXT.md`, `~/AI/arkai/docs/THREADS.md`, and `~/AI/arkai/docs/ai/SHARED-STATE.md` before doing work.
- Scan `~/AI/arkai/docs/ai/handoffs/` for recent entries.
- Do not edit `~/AI/arkai/docs/ai/SHARED-STATE.md` unless you are the current `Owner` in that file.
- If you are not the owner, write updates to a dated handoff in `~/AI/arkai/docs/ai/handoffs/`.

## 1) Non-Negotiables
- Do not invent APIs, configs, tools, or commands; verify from repo files first.
- Keep changes small and reversible.
- Never print, store, or commit secrets.
- Prefer read-only scans before edits on risky work.

## 2) Cross-Repo Governance
- Canonical authority: See `~/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md`.
- Do not duplicate authority matrices elsewhere. Other files should include a one-line pointer only.

## 3) AI Coordination (`docs/ai/`)
- Protocol and templates: `~/AI/arkai/docs/ai/README.md`
- Shared fast snapshot (single writer): `~/AI/arkai/docs/ai/SHARED-STATE.md`
- Handoffs: `~/AI/arkai/docs/ai/handoffs/YYYY-MM-DD-tool-summary.md`

## 4) Verification
- Run `~/AI/arkai/scripts/verify_ai_docs.sh` for docs/ai checks.
- Pre-commit hook enforces governance automatically when `docs/ai/` files are staged.
- Setup (one-time per clone): `git config core.hooksPath .githooks`
