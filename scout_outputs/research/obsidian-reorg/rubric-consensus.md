# Note-Taking System Rubric - Multi-Model Consensus

**Generated:** 2026-01-18
**Session:** 9e4fca9a
**Mode:** Deep adversarial debate
**Models:** Claude Opus 4.5, GPT-5.2, Gemini 3 Pro Preview

---

## Quick Consensus

**Confidence:** HIGH
**Agreement:** MAJORITY

The rubric is a strong baseline but requires three specific adjustments to be robust:

1. **Add 'Connection Discovery' and 'Cognitive Load' as criteria**
2. **Weight 'Capture' and 'Retrieval' as the dominant factors (combined ~50-60%)**
3. **Treat 'Modularity/Permissions' as a binary pass/fail gate rather than a scored scale**

---

## Summary

Experts agree that while the foundation is solid, the system misses the 'serendipity' of connecting notes and the 'cognitive load' impact of UI clutter. There is consensus that Capture and Retrieval are the critical survival metrics for any note-taking system and must be weighted highest. A significant methodological improvement is to treat security (Modularity) as a prerequisite gate, not a variable score.

---

## Original Criteria Under Review

1. **Capture friction (0-10)** - can you land in today's note instantly?
2. **Retrieval power (0-10)** - keyword + semantic search
3. **Surfacing (0-10)** - tasks, snoozes, time-bound resurfacing
4. **Modularity/permissions (0-10)** - AI can touch some folders, not all
5. **Maintainability (0-10)** - upgrades don't break you
6. **Mobile capture (0-10)** - voice -> text -> filed

---

## Individual Expert Perspectives

### Claude Opus 4.5

#### 1. Are These the Right Criteria?

**Answer**: Solid foundation (~85% complete), but add **Connection Discovery** as a 7th criterion.

**Reasoning**:
- Your current criteria cover capture -> store -> retrieve -> resurface, but miss **serendipitous rediscovery** (backlinks, graph view, "related notes")
- This is distinct from retrieval (active search) and surfacing (time-based) -- it's the "oh, I forgot I wrote about this" moments
- For ADHD, this compensates for poor working memory by letting the system make connections you forgot existed

**Minor consideration**: Fold "data portability" into Maintainability as a sub-criterion (can you export to plain markdown?).

#### 2. Weight Distribution

**Answer**: Weight by failure cost, not feature importance.

| Criterion | Weight | Why |
|-----------|--------|-----|
| **Capture friction** | 25 | Failure = thought lost forever. Non-negotiable for ADHD. |
| **Retrieval power** | 20 | "Findability over perfection" -- your stated philosophy |
| **Mobile capture** | 15 | Ambient ideas happen everywhere |
| **Surfacing** | 15 | Out of sight = out of mind for ADHD |
| **Connection discovery** | 10 | Nice-to-have insights, not critical path |
| **Modularity/permissions** | 10 | Arkai integration matters, but secondary |
| **Maintainability** | 5 | Migration pain is survivable |

#### 3. Scoring Methodology

**Answer**: Use **anchored 0-10 scales** with a **Usage Reality multiplier**.

**Formula**: `(Capability Score x Usage Reality) x Weight`

Where Usage Reality:
- **1.0** = actively use this feature
- **0.5** = occasionally use
- **0** = abandoned or never touched

**Example anchors for Capture Friction**:
- **10**: Single hotkey/tap -> today's note
- **5**: 2-3 actions (open app, navigate, start typing)
- **0**: 5+ steps or requires decision-making

**Caveat**: The Usage Reality multiplier is critical -- it prevents scoring theoretical capabilities you'll never use in practice. ADHD systems fail not from lack of features, but from friction that causes abandonment.

---

### GPT-5.2 (Chad)

#### 1. Are These the Right Criteria?

Your criteria are a strong "capture -> find -> resurface -> keep safe" baseline, but you're missing at least one ADHD-critical dimension: **connection/rediscovery (serendipity)**, plus a lightweight notion of **system trust** (does it reliably work when you're low-energy).

#### 2. What's Missing (Most Important Additions)

- **Connection/Rediscovery (0-10):** backlinks, "related notes," MOCs/indexes, random note, or AI "you've seen this before" prompts -- distinct from search (active) and surfacing (time-based).
- **System Trust/Reliability (0-10) (optional but valuable):** "does it work every time?" (sync conflicts, latency, broken plugins, capture-inbox always reachable). For ADHD, a system that *occasionally* fails often becomes a system you stop using.

#### 3. Weights (Optimized for 'Findability Over Perfection' + ADHD)

**If you keep 6 criteria:**
- Capture friction: 25-30%
- Retrieval power: 25-30%
- Surfacing: 10-15%
- Maintainability: 15-20%
- Modularity/permissions: 5-10%
- Mobile capture: 5-10%

**If you add Connection/Rediscovery as #7:**
- Capture: 25%
- Retrieval: 25%
- Maintainability: 15%
- Surfacing: 10%
- Connection: 10%
- Permissions: 10%
- Mobile: 5%

(Tune based on your actual usage)

#### 4. Most Robust Scoring Methodology

Use **anchored scoring** (clear 0/5/10 definitions per criterion) + **scenario-based tests** (e.g., "capture a thought in <10 seconds," "find that note from 3 months ago with 2 keywords," "AI can index Projects but not Personal"). Then compute a **weighted score** and do a quick **sensitivity check** ("if I change weights +/-10%, does the winner change?").

#### Caveat

If the rubric becomes another "system to maintain," it will backfire -- keep it fast to apply (10-15 minutes), re-score only after real-world use (e.g., 2 weeks), and treat results as directional rather than absolute.

---

### Gemini 3 Pro Preview

#### 1. Are These the Right Criteria?

The criteria are 90% correct but missing a **Cognitive Load** metric (visual noise/clutter assessment).

#### 2. Why Cognitive Load Matters

While previous experts suggested "Discovery," **Cognitive Load** is more critical for ADHD; a system with high "Surfacing" capabilities that floods you with 50 overdue tasks causes avoidance behavior. You need a metric for "how calm is the interface?"

#### 3. Weighting Recommendation

For the "messy collection" philosophy, **Capture Friction** (including Mobile) and **Retrieval Power** are the vital organs; if these fail, the system dies. Surfacing and Maintainability are secondary support systems.

Scoring should be weighted heavily toward **Capture** and **Retrieval** (60% combined).

#### 4. Methodology Recommendation

Replace abstract 0-10 scales with objective **"Time-to-Result"** benchmarks:
- Capture < 2 clicks
- Retrieval < 5 seconds
- Mobile sync < 1 minute

This provides concrete, non-emotional data points.

#### Critical Caveat

**Modularity/Permissions** should not be scored on a 0-10 scale but treated as a **binary Pass/Fail gate**; if the AI cannot reliably respect `.aiexclude` 100% of the time, the system is non-viable regardless of its other scores.

---

## Synthesized Recommendations

### Recommended Revised Rubric (8 Criteria)

| Criterion | Type | Weight | Description |
|-----------|------|--------|-------------|
| **Modularity/Permissions** | GATE | Pass/Fail | AI respects .aiexclude 100% - prerequisite |
| **Capture Friction** | Score | 25% | Single action to today's note |
| **Retrieval Power** | Score | 25% | Keyword + semantic search effectiveness |
| **Mobile Capture** | Score | 15% | Voice -> text -> filed workflow |
| **Surfacing** | Score | 10% | Tasks, snoozes, time-bound resurfacing |
| **Connection Discovery** | Score | 10% | Backlinks, "related notes," serendipity |
| **Cognitive Load** | Score | 10% | Interface calm, not overwhelming |
| **Maintainability** | Score | 5% | Upgrades don't break you, portability |

### Recommended Scoring Methodology

1. **Binary Gate First**: Does the system pass Modularity/Permissions? If no, disqualify.

2. **Anchored Scales**: Define concrete 0/5/10 anchors for each criterion:
   - 10 = Best-in-class, single action
   - 5 = Acceptable, 2-3 actions
   - 0 = Unacceptable friction

3. **Time-Based Benchmarks** (where applicable):
   - Capture: < 2 clicks/taps
   - Retrieval: < 5 seconds
   - Mobile sync: < 1 minute

4. **Usage Reality Multiplier**:
   - 1.0 = actively using
   - 0.5 = occasionally using
   - 0.0 = abandoned/theoretical

5. **Final Formula**: `Sum of (Score x Usage x Weight)`

6. **Sensitivity Check**: If changing weights +/-10% changes the winner, investigate further.

7. **Meta-Rule**: Keep scoring fast (10-15 min). Re-score only after 2 weeks of real use.

---

## Key Takeaways

1. **Capture + Retrieval are survival metrics** (~50-60% weight combined) - if these fail, the system dies regardless of other features

2. **Modularity/Permissions is binary** - either AI respects boundaries 100% or the system is non-viable

3. **Add Connection Discovery** - compensates for ADHD working memory by surfacing "forgotten" connections

4. **Consider Cognitive Load** - high capability + overwhelming UI = system abandonment

5. **Avoid analysis paralysis** - use concrete benchmarks, not abstract scales; treat results as directional, not absolute

6. **Usage Reality matters** - score what you actually use, not theoretical capabilities

---

*Generated via steelman:consensus multi-model debate*
