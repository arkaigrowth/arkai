# Steelman Consensus Log

Decision history from multi-model consensus queries.

---
## 2026-01-10 22:20 - "ARCHITECTURE REVIEW for arkai - an AI orchestration CLI in Rust.

CURRENT STATE:
- arkai = Rust spine: event-sourced, resumable pipelines, content library (catalog + storage)
- fabric = Go CLI: 240+ AI patterns (extract_wisdom, summarize, etc), YouTube/web fetching, LLM access
- arkai wraps fabric via subprocess (fabric -p pattern, fabric -y url, fabric -u url)
- Three special actions: __youtube__, __web__, and pattern names

PROPOSED ADDITIONS:
1. Add __podcast__ action: yt-dlp download → fabric --transcribe-file → pattern chain
2. Add natural language interface: 'arkai ask ...' for intent → command translation
3. Three-layer brain: Layer1=reflexes (URL regex), Layer2=learned mappings, Layer3=LLM fallback
4. Integrate RAGFlow for semantic search (separate Docker service)

KEY JUDGMENT CALLS:
A) Monorepo (fork fabric into arkai) vs wrapper (current) vs pure orchestrator?
B) NL interface: thin wrapper to fabric suggest_pattern vs own intent engine vs hybrid?
C) Where does podcast workflow logic live - arkai or contribute upstream to fabric?
D) RAG: build into arkai vs integrate with RAGFlow vs stay with keyword search?
E) Is the three-layer brain (reflex→learned→LLM) sound architecture or overengineered?

DESIGN PRINCIPLES WE WANT:
- Clear separation of concerns (no fuzziness between arkai and fabric)
- Bias for determinism (LLM is fallback, not primary path)
- Unix philosophy (composable, text interfaces, one thing well)
- Security (no secrets in events, sandboxed execution)
- Low friction for users (minimal config, sensible defaults)

Evaluate critically. What's the SOTA approach? Where are the pitfalls? What would you change?"
**Confidence:** HIGH | **Agreement:** MAJORITY

**Consensus:** Maintain arkai as a pure orchestrator (do not fork) using a strict adapter pattern to interface with fabric. Implement the 3-layer brain (Reflex → Learned → LLM) and host the podcast pipeline logic within arkai. For RAG, prioritize an embedded, Rust-native solution (e.g., SQLite-vec or LanceDB) to preserve the 'low friction' design principle, treating RAGFlow only as an optional external provider.

The experts unanimously agree that arkai should remain a distinct orchestrator managing state and workflows, while fabric serves as the stateless execution engine. The primary divergence lies in the RAG implementation: while one expert suggested RAGFlow, the consensus leans heavily toward embedded vector search to avoid Docker dependencies and maintain the 'single binary' utility philosophy.

---

---
## 2026-01-18 23:30 - "Evaluate this scoring rubric for note-taking systems. The proposed criteria are:

1. Capture friction (0-10) - can you land in today's note instantly?
2. Retrieval power (0-10) - keyword + semantic search
3. Surfacing (0-10) - tasks, snoozes, time-bound resurfacing
4. Modularity/permissions (0-10) - AI can touch some folders, not all
5. Maintainability (0-10) - upgrades don't break you
6. Mobile capture (0-10) - voice → text → filed

Questions:
1. Are these the right criteria? What's missing?
2. How should the weights be distributed? (Should any be weighted higher?)
3. What scoring methodology would be most robust?"
**Confidence:** HIGH | **Agreement:** MAJORITY

**Consensus:** The rubric is a strong baseline but requires three specific adjustments to be robust: 1) Add 'Connection Discovery' and 'Cognitive Load' as criteria, 2) Weight 'Capture' and 'Retrieval' as the dominant factors (combined ~50-60%), and 3) Treat 'Modularity/Permissions' as a binary pass/fail gate rather than a scored scale.

Experts agree that while the foundation is solid, the system misses the 'serendipity' of connecting notes and the 'cognitive load' impact of UI clutter. There is consensus that Capture and Retrieval are the critical survival metrics for any note-taking system and must be weighted highest. A significant methodological improvement is to treat security (Modularity) as a prerequisite gate, not a variable score.

---

