# arkai TODO

> **Last Updated**: 2026-01-21

---

## 🔥 Immediate (Delegate to arkai-voice-builder)

### Voice Capture Phase 1
- [ ] Implement watcher (notify crate) for `~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/`
- [ ] Implement queue manager (JSONL, idempotent)
- [ ] Add CLI: `arkai ingest voice status`
- **Reference**: `docs/ARKAI_VOICE_CAPTURE_DESIGN.md` Section "Phase 1: Foundation"

### Spec Kernel Integration (Chad's v3)
- [ ] Unzip `arkai-spec-bootstrap-v3.zip` to `spec/`
- [ ] Review ALIGNMENT.md for Rust ↔ Spec gaps
- [ ] PR-1: Add `schema_version: Option<String>` to Event struct

---

## 🎯 Next Up

### Gmail Triage (Separate Session)
- [ ] Create `arkai-gmail` repo
- [ ] OAuth setup
- [ ] Implement Layer A (ingestion)
- **Reference**: `docs/ARKAI_GMAIL_DESIGN.md`

### Clawdbot Integration (In Progress)
- [x] VPS setup (Hetzner CAX21, Tailscale, 24/7)
- [x] Claudia personality (SOUL.md, ARKAI.md uploaded)
- [x] Voice transcription (OpenAI Whisper API)
- [x] Tools lockdown (read/write/edit only)
- [x] Clone arkai to VPS for L1 (Observer) mode ✅ 2026-01-25
- [x] Clone fabric-arkai to VPS ✅ 2026-01-25
- [x] GitHub SSH key on VPS ✅ 2026-01-25
- [x] Telegram sender (`arkai voice process`) ✅ 2026-01-25
- [x] `tell-claudia` fish function ✅ 2026-01-25
- [ ] iOS Shortcut voice trigger (Back Tap not registering)
- [ ] ElevenLabs TTS for Claudia responses
- [ ] Wire clawdbot → arkai CLI triggers
- **Session Log**: `~/AI/clawdbot/sessions/2026-01-25-clawdbot-setup.md`
- **Handoff**: `.ralph/memory/handoffs/2026-01-25-clawdbot-l1-voice-integration.md`

---

## ✅ Completed (This Session)

- [x] RALPH session memory system (`.ralph/`, `scripts/ralph`)
- [x] Gmail triage design doc (~2200 lines)
- [x] Voice capture design doc (~800 lines)
- [x] Repo tree for Chad (`scout_outputs/REPO_TREE_2026-01-21.md`)
- [x] Spec kernel Q&A (enums, JSONL storage, schema_version)

---

# Future Enhancements

## ✅ Config File Support (Implemented)
`.arkai/config.yaml` is now supported with:
- Path configuration (home, library, content_types)
- Safety limits (max_steps, timeout, max_input_size)
- Fabric integration settings
- Config file discovery (searches current dir and parents)
- Env var overrides still work (highest priority)

## Potential Future Work

### Title Extraction from YouTube
Currently uses video ID as title. Could extract actual title from:
- yt-dlp metadata
- YouTube API
- Transcript content analysis

### Web Article Support
- Improve web content extraction (readability)
- Better title extraction from HTML

### Batch Processing
- Process multiple URLs from a file
- Playlist URL support

### Export/Import
- Export library to markdown
- Import from other knowledge bases
