# LinkedIn Pitches: arkai + Fabric Stack

Quick-reference pitches for sharing the personal AI infrastructure stack.

---

## The Unified Positioning

| Layer | What | Why |
|-------|------|-----|
| **Fabric** | 240+ AI prompts | The *muscle* — knows what to do |
| **arkai** | Rust orchestration | The *spine* — never forgets, never fails |
| **Together** | Personal AI infrastructure | *Second brain* that actually works |

**Core metaphor**: "Your AI needs a spine"

---

## LinkedIn DM Version (Cold Outreach)

> **Subject: Your AI workflows losing results?**
>
> Hey [Name],
>
> I built something that might interest you — a personal AI infrastructure stack:
>
> **Fabric** = 240+ crowdsourced AI prompts (extract insights from any video in 30 seconds)
> **arkai** = Rust spine that never loses your work (event-sourced, resumable, searchable)
>
> Together: Every YouTube video, article, or podcast becomes permanent searchable knowledge.
>
> Quick demo: `arkai ingest "any-youtube-url"` → wisdom extracted, cataloged, searchable forever.
>
> Happy to show you if you're building AI workflows. No pitch, just nerding out.

**Character count**: ~520

---

## LinkedIn Post Version (Feed)

> **Your AI results vanish. Mine don't.**
>
> I kept running into the same problem:
> • Watch a 2-hour podcast → insights gone next session
> • Pipeline fails halfway → start over from scratch
> • "What did the AI do?" → 🤷
>
> So I built a stack that fixes this:
>
> **Layer 1: Fabric** (github.com/danielmiessler/fabric)
> 240+ crowdsourced AI prompts. extract_wisdom, summarize, analyze_claims.
> Turn any content into exactly what you need.
>
> **Layer 2: arkai** (Rust spine)
> Event-sourced orchestration. Never lose results. Resume from failures. Full audit trail.
>
> **Together:**
> ```
> arkai ingest "youtube-url" --tags "ai"
> arkai search "transformers"
> ```
> Every video, article, podcast → permanent searchable knowledge base.
>
> The insight that changed everything:
> *"A smart AI with a bad system loses to a well-designed system with a dumber model."*
>
> Your AI needs a spine. Build one.
>
> 🔗 Links in comments

**Character count**: ~950 (optimal for LinkedIn engagement)

---

## One-Liners (for bios, intros, etc.)

**Technical:**
> "Fabric handles AI transformation. arkai handles reliability. Together: personal AI infrastructure that never loses your work."

**Casual:**
> "I built a system that turns any YouTube video into searchable knowledge in 30 seconds — and never forgets it."

**Provocative:**
> "Your AI results vanish after every chat. Mine don't. Here's why."

---

## 5 High-Leverage Patterns (for demos)

| Pattern | One-Liner | Demo |
|---------|-----------|------|
| `extract_wisdom` | 2-hour video → 2-min insights | `arkai ingest "youtube-url"` |
| `summarize` | 50 pages → 5 bullets | `cat doc.txt \| fabric -p summarize` |
| `improve_writing` | Instant editor | `echo "draft" \| fabric -p improve_writing` |
| `analyze_claims` | Fact-check anything | `fabric -p analyze_claims < article.txt` |
| `rate_content` | Worth my time? | `fabric -y "url" -p rate_content` |

---

## The Killer Demo

**Lead with `extract_wisdom` on any YouTube video.**

Why this works:
1. Everyone has videos they "should watch but don't have time for"
2. Works immediately with any URL they care about
3. Output is genuinely useful (not just summary — actionable insights)
4. Takes 30 seconds, instantly understood

```bash
arkai ingest "https://youtube.com/watch?v=ANY_VIDEO" --tags "demo"
arkai show <id> --full
```

---

## Links for Comments

```
Fabric: https://github.com/danielmiessler/fabric
arkai: https://github.com/arkaigrowth/arkai
```

---

*"A really smart AI with a bad system is way worse than a well-designed system with a less smart model."*
