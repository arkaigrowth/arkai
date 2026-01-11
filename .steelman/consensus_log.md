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

