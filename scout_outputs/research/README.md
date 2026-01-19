# Diarization + Keyframes Research

> **Started**: 2026-01-17 | **Session**: 5 | **Goal**: Enhance arkai transcripts with speaker ID + visual context

---

## Research Scope

### 1. Speaker Diarization
**Goal:** Add `[Speaker Name]` tags to transcripts so we know WHO is speaking.

| Option | Type | Cost | Pros | Cons |
|--------|------|------|------|------|
| WhisperX | Local | Free | Privacy, no API limits | Requires GPU, complex setup |
| AssemblyAI | Cloud API | ~$0.006/min | Excellent accuracy, simple API | Cost, data leaves machine |
| Deepgram | Cloud API | Varies | Fast, good quality | Less diarization focus |

### 2. Keyframe Extraction
**Goal:** Extract visual context at key moments (speaker changes, topic shifts, every N seconds).

**Discovery:** video-download skill already integrates with video-ops skill for keyframes!
- Script: `extract_keyframes.py` from video-ops
- Flag: `--extract-keyframes` on download

### 3. Integration Architecture
How should these pieces connect to arkai's existing pipeline?

```
Current Flow:
  URL → fabric -y (transcript) → extract_wisdom → summary → library

Enhanced Flow (proposed):
  URL → video-download (+ keyframes) → diarization → fabric patterns → library
```

---

## Research Output Structure

```
research/
├── README.md              ← You are here
├── diarization/
│   ├── whisperx.md        ← Local option research
│   └── assemblyai.md      ← Cloud option research
├── keyframes/
│   └── video-ops.md       ← video-ops skill analysis
└── SYNTHESIS.md           ← Final recommendation (after all research)
```

---

## Key Questions to Answer

### Diarization
1. What hardware does WhisperX require? (GPU? RAM? Install complexity?)
2. What's AssemblyAI's actual per-minute cost with diarization enabled?
3. Can diarization work with existing fabric transcripts, or need audio file?
4. Output format: How are speakers labeled? Can we get names or just "Speaker 1"?

### Keyframes
1. What does video-ops extract_keyframes.py output?
2. Can we extract at speaker changes (requires diarization first)?
3. What's the optimal density for our use case?
4. How do keyframes integrate with markdown transcripts?

### Integration
1. Should video-download become the "ingestion frontend" for arkai?
2. Where in pipeline does diarization fit? (before or after fabric?)
3. How to store keyframes in library folder structure?

---

## Subagent Assignments

| Agent | Focus | Output File |
|-------|-------|-------------|
| WhisperX Researcher | Local diarization | diarization/whisperx.md |
| AssemblyAI Researcher | Cloud diarization | diarization/assemblyai.md |
| Keyframes Analyst | video-ops capabilities | keyframes/video-ops.md |

---

## Target Output Format (Enhanced Transcript)

```markdown
[00:00:02] [Daniel] Hey, what's up? So I want to talk about AI agents.
[KEYFRAME: keyframes/0002.jpg]
[00:00:08] [Mike] Yeah, this is fascinating stuff. Let me show you...
[00:00:15] [Daniel] Right, so the key insight here is...
[KEYFRAME: keyframes/0015.jpg]
```

This format enables:
- Timestamp navigation
- Speaker attribution
- Visual context at key moments
- Searchable by speaker name
