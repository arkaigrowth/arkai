# Codex Handoff: Fabric Binary Override + Doctor Diagnostics

Date: 2026-04-05
Branch: `codex/issue-1-fabric-binary-override-doctor`
Worktree: `/tmp/arkai-issue-1-fabric-binary-override-doctor`

## Summary

Implemented Arkai issue #1 as a focused follow-up to `6dd2087`:

- Added an explicit Fabric binary override via `ARKAI_FABRIC_BIN`
- Added a matching config override via `.arkai/config.yaml` at `fabric.binary`
- Added Fabric binary diagnostics to `arkai doctor --json`
- Rejected incompatible `fabric` binaries early with a clear error message

Scope intentionally stayed narrow:

- No pipeline redesign
- No OpenClaw changes
- No YouTube VTT/fetch changes for this issue

## Files Changed

- `src/config/mod.rs`
- `src/adapters/fabric.rs`
- `src/cli/mod.rs`
- `README.md`

## Behavior

### Override resolution

Fabric binary selection now resolves in this order:

1. `ARKAI_FABRIC_BIN`
2. `.arkai/config.yaml` -> `fabric.binary`
3. Auto-detect `fabric-ai`
4. Auto-detect `fabric`

### Doctor output

`arkai doctor --json` now reports:

- `fabric.requested_binary`
- `fabric.selected_binary`
- `fabric.selection_source`
- `fabric.signature_passed`
- `fabric.argv0_alias`
- `fabric.error`

If the selected binary does not match the AI Fabric CLI signature, doctor reports:

- `health.status = "fail"`
- an error entry under `health.issues`

### Runtime rejection

Pattern execution now fails fast when the selected binary is incompatible, instead of falling through to a later or ambiguous failure.

## Verification Commands

### Compile

```bash
cargo check --quiet
```

### Focused tests

```bash
cargo test --quiet test_config_file_parsing
cargo test --quiet test_resolve_fabric_binary_override_prefers_env
cargo test --quiet test_resolve_fabric_binary_override_uses_config
cargo test --quiet test_explicit_binary_accepts_compatible_ai_fabric_help
cargo test --quiet test_explicit_binary_rejects_incompatible_help
```

### Normal doctor path

```bash
cargo run --quiet -- doctor --json
```

Observed result:

- `selected_binary`: `/opt/homebrew/bin/fabric-ai`
- `selection_source`: `auto_path_fabric_ai`
- `signature_passed`: `true`

### Env override failure path

Create a fake incompatible binary:

```bash
printf '%s\n' '#!/bin/sh' 'echo "Usage: fabric [options]"' > /tmp/arkai-fake-fabric
chmod +x /tmp/arkai-fake-fabric
```

Check doctor:

```bash
ARKAI_FABRIC_BIN=/tmp/arkai-fake-fabric cargo run --quiet -- doctor --json
```

Observed result:

- `selection_source`: `env_override`
- `signature_passed`: `false`
- `health.status`: `fail`
- `fabric.error` clearly reports the binary is incompatible and expected the AI Fabric CLI help signature containing `--pattern`, `--youtube`, and `--scrape_url`

Check runtime rejection:

```bash
printf 'hello from issue 1 verification\n' | ARKAI_FABRIC_BIN=/tmp/arkai-fake-fabric cargo run --quiet -- pattern summarize
```

Observed result:

- command exits non-zero
- Arkai reports `Failed to run pattern 'summarize'`
- root cause is the same explicit incompatible-binary message

### Config override failure path

Create a temp config:

```bash
mkdir -p /tmp/arkai-issue-1-config-check/.arkai
printf '%s\n' 'fabric:' '  binary: /tmp/arkai-fake-fabric' > /tmp/arkai-issue-1-config-check/.arkai/config.yaml
```

Run doctor from that directory:

```bash
cargo run --quiet --manifest-path /tmp/arkai-issue-1-fabric-binary-override-doctor/Cargo.toml -- doctor --json
```

from cwd:

```bash
/tmp/arkai-issue-1-config-check
```

Observed result:

- `selection_source`: `config_override`
- `config_file`: `/private/tmp/arkai-issue-1-config-check/.arkai/config.yaml`
- `signature_passed`: `false`

### AI docs verification

```bash
./scripts/verify_ai_docs.sh
```

Observed result:

- fails in this clean worktree with `Missing docs/ai/SHARED-STATE.md`
- this appears to be a baseline property of the `6dd2087` checkout used for the fresh worktree, not a regression introduced by this issue branch

## Residual Risks

- `doctor` is intentionally minimal in this branch and only reports the Fabric/path-health slice needed for this issue.
- Signature detection relies on `--help` containing AI Fabric markers (`--pattern`, `--youtube`, `--scrape_url`). If upstream AI Fabric changes its help output, this heuristic may need an update.
- `ingest_youtube` now uses `FabricAdapter::new()` for pattern execution in this branch, which keeps behavior aligned with override selection, but broader ingest/doctor UX is still intentionally out of scope here.
- `./scripts/verify_ai_docs.sh` remains red in this worktree until the baseline missing `docs/ai/SHARED-STATE.md` situation is addressed separately.
