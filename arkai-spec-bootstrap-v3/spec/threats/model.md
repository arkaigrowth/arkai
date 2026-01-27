# Threat Model (Bootstrap)
Threats:
- prompt injection
- data exfiltration
- privilege escalation
- silent side effects
- behavioral harm (nag loops)

Mitigations:
- compartments (Quarantine→Sanitizer→Reader→Critic→Executor)
- schemas + whitelists + rate caps
- replay-first observability (JSONL)
- consent-first nudges
