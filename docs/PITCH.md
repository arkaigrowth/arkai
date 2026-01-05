# The AI OS Stack: Rust Spine + Fabric Patterns

## One-Liner

> **Separate state from intelligence.** Rust handles reliability, Fabric handles AI.

---

## The Stack (30-second version)

```
    ┌─────────────────────────────────────┐
    │      Natural Language (Claude)      │  ← "ingest this video"
    └───────────────┬─────────────────────┘
                    ▼
    ┌─────────────────────────────────────┐
    │       arkai (Rust Spine)            │  ← State + Orchestration
    │                                     │
    │  • Event-sourced (resumable)        │
    │  • Idempotent (safe to retry)       │
    │  • Auditable (full history)         │
    └───────────────┬─────────────────────┘
                    ▼
    ┌─────────────────────────────────────┐
    │       fabric (Patterns)             │  ← AI Transformation
    │                                     │
    │  • 200+ community prompts           │
    │  • YouTube/web fetching             │
    │  • extract_wisdom, summarize, etc.  │
    └───────────────┬─────────────────────┘
                    ▼
    ┌─────────────────────────────────────┐
    │       LLM (Claude, GPT, etc.)       │  ← Raw Intelligence
    └─────────────────────────────────────┘
```

---

## Why This Beats Raw ChatGPT

| Feature | ChatGPT | arkai + Fabric |
|---------|---------|----------------|
| Results persist | ❌ Gone after chat | ✅ Searchable library |
| Failed? Resume | ❌ Start over | ✅ Pick up where you left off |
| What happened? | ❌ Who knows | ✅ Full event log |
| Repeatable | ❌ Different each time | ✅ Same input = same ID |

---

## The Insight

*"A really smart AI with a bad system is way worse than a well-designed system with a less smart model."*

**Your AI needs a spine.** Something that:
- Remembers what it did
- Can recover from failures
- Won't lose your work
- Grows smarter as your library grows

---

## Get Started (5 minutes)

```bash
# Install
cargo install --git https://github.com/arkaigrowth/arkai
go install github.com/danielmiessler/fabric@latest && fabric --setup

# Ingest a video
arkai ingest "https://youtube.com/watch?v=..." --tags "ai"

# Search your knowledge
arkai search "transformers"

# Your library grows. Your system remembers. You win.
```

---

## Project Structure

```
your-project/
├── .arkai/              # Engine state (gitignore)
│   ├── config.yaml
│   └── runs/
├── library/             # Your knowledge (git-track!)
│   ├── youtube/
│   └── articles/
```

---

## Key Principles

1. **Event Sourcing** — Every action logged, state reconstructible
2. **Fail-Fast** — Timeouts, limits, denylist patterns
3. **Content-Addressable** — SHA256(url) = deterministic IDs
4. **Unix Philosophy** — Small tools that compose

---

*Build your second brain with a spine that doesn't break.*
