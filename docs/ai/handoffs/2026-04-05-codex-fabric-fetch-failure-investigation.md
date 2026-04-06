# Handoff: Fabric Fetch Failure Investigation

Date: 2026-04-05
From: Codex
To: SysOps / future Arkai sessions

## Summary

Recent `web-wisdom` / `youtube-wisdom` run failures are **not** caused by the replay/input-persistence patch.
Pipeline-name reconstruction is working in current source, and current-source runs persist `input.txt` plus
structured `RunStarted.payload`.

The true failures were in the Fabric fetch subprocess:

- `web-wisdom` recent failures (`aa54c9fe...`, `af115cdc...`) came from invoking Homebrew `fabric-ai`
  as `fabric-ai`. Its fetch mode misbehaves in that form and exits with:
  `could not get pattern fabric-ai: pattern 'fabric-ai' not found`
- `youtube-wisdom` still fails on the older caption path:
  `no VTT files found in directory`

## Root Cause

Two separate issues existed:

1. **`fabric-ai` argv[0] bug / packaging quirk**
   - `/opt/homebrew/bin/fabric-ai -u https://example.com`
     reproduced the exact run-log failure:
     `could not get pattern fabric-ai: pattern 'fabric-ai' not found`
   - Forcing the same binary to run as `fabric` fixes web fetch:
     `exec -a fabric /opt/homebrew/bin/fabric-ai -u https://example.com`
   - This indicates the Homebrew AI Fabric binary expects `argv[0] == "fabric"` for fetch mode.

2. **Unsafe fallback to `fabric`**
   - On this host, `fabric` resolves to the unrelated Python SSH/Lightning tool:
     `/Users/alexkamysz/.pyenv/versions/3.11.9/bin/fabric`
   - It does **not** support `-p`, `-u`, or `-y`.
   - The previous adapter logic would fall back to that binary if `fabric-ai --help` was unavailable,
     which is not a safe or meaningful fallback.

## Fix Applied

File changed:
- `src/adapters/fabric.rs`

Changes:
- Added AI-Fabric-specific binary detection based on `--help` signature
  (`--pattern`, `--youtube`, `--scrape_url`)
- Stopped treating arbitrary `fabric` binaries as valid unless they match that signature
- Normalized `argv[0]` to `fabric` whenever the selected binary path basename is `fabric-ai`
- Routed subprocess, web fetch, YouTube fetch, and health check through the same normalized command helper
- Added unit tests for:
  - AI Fabric help detection
  - rejection of non-AI `fabric`
  - `fabric-ai` argv aliasing

## Verification

### Installed/source drift

Before rebuild:
- `~/.cargo/bin/arkai doctor --json` -> `unrecognized subcommand 'doctor'`
- `./target/release/arkai doctor --json` -> same

After rebuild + install:
- `cargo run -- doctor --json` works
- `./target/release/arkai doctor --json` works
- `~/.cargo/bin/arkai doctor --json` works

### Direct Fabric repro

- `/opt/homebrew/bin/fabric-ai -u https://example.com`
  -> `could not get pattern fabric-ai: pattern 'fabric-ai' not found`
- `bash -lc 'exec -a fabric /opt/homebrew/bin/fabric-ai -u https://example.com'`
  -> succeeds, returns Example Domain markdown
- `/opt/homebrew/bin/fabric-ai -y https://youtu.be/TqjmTZRL31E --transcript-with-timestamps`
  -> `no VTT files found in directory`
- `bash -lc 'exec -a fabric /opt/homebrew/bin/fabric-ai -y https://youtu.be/TqjmTZRL31E --transcript-with-timestamps'`
  -> still `no VTT files found in directory`

### Current-source / installed Arkai behavior

- `cargo run -- run web-wisdom --input /tmp/arkai-web-url.txt`
  -> fetch step completes; run proceeds into `wisdom`
- `~/.cargo/bin/arkai run web-wisdom --input /tmp/arkai-web-url.txt`
  -> fetch step completes; run proceeds into `wisdom`
- `cargo run -- run youtube-wisdom --input /tmp/arkai-youtube-url.txt`
  -> still fails in fetch with `no VTT files found in directory`

### Replay/input persistence

Current-source run `3212494e-470f-4ee0-b7d0-336055a6d633` / later runs show:
- `input.txt` persisted under `~/.arkai/runs/<id>/input.txt`
- `RunStarted.payload` includes `pipeline_name`, `input_bytes`, `input_sha256`

That means the replay/input-persistence patch is functioning in current source.
Older failed runs without payload came from the stale installed binary, not from a new regression.

## Sandbox vs Host

Sandbox-vs-host differences are **not** the main explanation for the recent Arkai run failures anymore.

- In the sandbox, `fabric-ai -u https://example.com` fails earlier on DNS (`r.jina.ai: no such host`)
- On the host, the same command reproduced the real Arkai failure
- After the adapter fix, host `web-wisdom` fetch succeeds

So the relevant differences now are:
- **stale installed binary vs current source**
- **host `fabric-ai` CLI behavior vs wrong `fabric` fallback**

Not the replay patch, and not Docker-style sandboxing.

## Remaining Issue

`youtube-wisdom` still depends on Fabric's transcript/VTT path and still fails for videos where captions
cannot be fetched:

- `YouTube fetch failed with exit code 1: no VTT files found in directory`

This is separate from the adapter/argv fix. The existing durable workaround in Arkai is the direct
yt-dlp audio + Whisper ingest path, not the `youtube-wisdom` pipeline fetch step.
