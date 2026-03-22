# AI Coordination Protocol

Shared workspace for all AI agents operating in this repo.
Git-tracked. Tool-agnostic.

## Directory Structure

```
docs/ai/
  SHARED-STATE.md     Fast snapshot (single writer)
  authority/          Cross-repo authority maps
  research/           Investigations and reusable findings
  decisions/          Architecture Decision Records (ADRs)
  handoffs/           Cross-session and cross-tool handoffs
  migration/          Legacy migration inventories
```

## Session Protocol

On session start:
1. Read `/Users/alexkamysz/AI/arkai/docs/CONTEXT.md` and `/Users/alexkamysz/AI/arkai/docs/THREADS.md`.
2. Read `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md`.
3. Scan `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/` for recent entries.

During work:
1. If you are not the `Owner` in `SHARED-STATE.md`, do not edit it.
2. Non-owner updates must include a handoff file in `docs/ai/handoffs/`.
3. Keep files concise and dated. Never include secrets.

On session end:
1. Write a handoff when work continues in another session/tool.
2. Owner thread rolls accepted updates into `SHARED-STATE.md`.
3. Keep state-sync commits isolated.

## Verification

Run:

```bash
/Users/alexkamysz/AI/arkai/scripts/verify_ai_docs.sh
```

Hard checks include:
- `SHARED-STATE.md` must not be a symlink.
- Non-owner threads must not modify `SHARED-STATE.md`.
- Non-owner `docs/ai` updates must include a handoff under `docs/ai/handoffs/`.
- If `SHARED-STATE.md` is staged, state-sync commits must be isolated.

## State-Sync Commit Isolation

Use an isolated commit for shared-state updates:

```bash
git add docs/ai/SHARED-STATE.md
git commit --only /Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md -m "docs(ai): sync shared state"
```

## Cross-Repo Rule

Use canonical + pointer:
- Keep canonical authority in `docs/ai/authority/CROSS-REPO-AUTHORITY.md`.
- Other files must only point to that canonical map.
