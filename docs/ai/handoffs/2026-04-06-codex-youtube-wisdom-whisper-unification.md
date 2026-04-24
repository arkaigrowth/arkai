# Codex Handoff: youtube-wisdom Whisper Unification

Date: 2026-04-06
Branch: `codex/issue-2-youtube-whisper-unification`
Worktree: `/tmp/arkai-issue-2-publish`

## Summary

- `youtube-wisdom` no longer depends on Fabric's caption/VTT fetch path.
- The `__youtube__` adapter action now uses Arkai's durable `yt-dlp` audio + Whisper transcript path.
- Pipeline runs now preserve `transcript.txt` plus `transcript.json` when Whisper emits it.
- Direct `arkai ingest --content-type youtube` now uses the same shared acquisition helper.

## Files Changed

- `src/ingest/youtube.rs`
- `src/ingest/mod.rs`
- `src/adapters/mod.rs`
- `src/adapters/fabric.rs`
- `src/core/event_store.rs`
- `src/core/orchestrator.rs`
- `src/cli/mod.rs`
- `pipelines/youtube-wisdom.yaml`

## Verification

### Build and tests

```bash
cargo check --quiet
cargo test --quiet store_named_artifact_under_step_directory
cargo test --quiet preserves_json_artifact
cargo test --quiet allows_missing_json_artifact
cargo test --quiet
```

### Previously failing youtube-wisdom URL

```bash
printf 'https://youtu.be/TqjmTZRL31E\n' > /tmp/arkai-youtube-url.txt
cargo run --quiet -- run youtube-wisdom --input /tmp/arkai-youtube-url.txt
```

Successful run:

- `d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4`

Observed event timing:

- `fetch` completed in `499839ms`
- `wisdom` completed in `74076ms`
- `summary` completed in `11067ms`

Verified run artifacts:

- `~/.arkai/runs/d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4/artifacts/fetch.md`
- `~/.arkai/runs/d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4/artifacts/fetch/transcript.txt`
- `~/.arkai/runs/d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4/artifacts/fetch/transcript.json`
- `~/.arkai/runs/d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4/artifacts/wisdom.md`
- `~/.arkai/runs/d686e05d-2bc5-4f21-ae2a-fb9cd6be7eb4/artifacts/summary.md`

### Direct ingest on temp state

```bash
ARKAI_HOME=/tmp/arkai-issue2-home \
ARKAI_LIBRARY=/tmp/arkai-issue2-library \
cargo run --quiet -- ingest https://youtu.be/TqjmTZRL31E --content-type youtube --title 'Issue 2 verification'
```

Verified output directory:

- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)`

Verified artifacts:

- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)/transcript.txt`
- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)/transcript.json`
- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)/wisdom.md`
- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)/summary.md`
- `/tmp/arkai-issue2-library/youtube/Issue 2 verification (TqjmTZRL31E)/claims.json`

### AI docs verification

```bash
./scripts/verify_ai_docs.sh
```

Observed result:

- fails in this clean worktree with `Missing docs/ai/SHARED-STATE.md`
- this is a baseline property of the current remote base, not an issue-2 code regression

## Residual Risks

- The durable audio + Whisper path is much slower than the old caption/VTT fetch. On the verification URL, the `fetch` step took about 8 minutes 20 seconds, so `pipelines/youtube-wisdom.yaml` was bumped from `120s` to `600s`.
- Very long videos may still need a higher timeout budget.
- The helper still assumes local binaries at `/opt/homebrew/bin/yt-dlp` and `/opt/homebrew/bin/whisper`, matching the existing direct-ingest path.
- I did not touch OpenClaw, SysOps, OCR/keyframes/search/tagging work, or the separate `web-wisdom` fetch regression.
- `./scripts/verify_ai_docs.sh` remains red until the repo baseline missing `docs/ai/SHARED-STATE.md` situation is addressed separately.
