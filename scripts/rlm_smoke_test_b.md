# RLM Smoke Test B: Realistic + Adversarial

> **Purpose**: Prove chunking, batch subquery, evidence candidate output, sandbox security
> **Paste this into Claude Code after restart**

---

## Test Prompt

```
Run RLM Smoke Test B (realistic + adversarial):

## Part 1: Directory Analysis

1. Load arkai source directory as context:
   - Read all .rs files from ~/AI/arkai/src/ (use Glob + Read tools)
   - Concatenate into single context
   - Use rlm_load_context with name="arkai_src"

2. Chunk the context:
   - Use rlm_chunk_context with strategy="lines", size=200
   - Note the chunk count

3. Filter for security-relevant patterns:
   - Use rlm_filter_context with pattern: `unsafe|TODO|FIXME|panic!|unwrap\(|expect\(|sql|auth|token`
   - Output name: "security_hits"

4. Get top 3 hit chunks:
   - Use rlm_get_chunk for indices 0, 1, 2 of "security_hits" (or fewer if less exist)

## Part 2: Batch Sub-Query (OpenRouter)

5. Run batch analysis on chunks using OpenRouter:
   - Use rlm_sub_query_batch with:
     - provider: "openrouter"
     - model: "openai/gpt-4o-mini"
     - query: "Is this code security-relevant? If yes, provide a verbatim quote as evidence candidate. Format: {\"relevant\": bool, \"quote\": \"exact text\" or null, \"reason\": \"explanation\"}"
     - chunk_indices: [0, 1, 2] (or available indices)

## Part 3: Output Artifacts

6. Write findings.json:
   - Aggregate sub-query results
   - Include chunk metadata

7. Write evidence_candidates.jsonl:
   - One line per quote with:
     - quote (verbatim from chunk)
     - chunk_id
     - chunk_strategy: "lines:200@v1"
     - source_kind: "repo"
     - verification_status: "unverified"

8. Write manifest.json (arkai skill format):
   {
     "skill": "rlm-smoke-test-b",
     "success": true,
     "artifacts": [
       {"name": "findings.json", "type": "structured", "path": "./findings.json"},
       {"name": "evidence_candidates.jsonl", "type": "evidence", "path": "./evidence_candidates.jsonl"}
     ],
     "result": {"chunks_analyzed": N, "evidence_candidates": M, "provider": "openrouter"}
   }

## Part 4: Sandbox Security Proof

9. Test sandbox blocks dangerous imports:
   - Use rlm_exec with code: `import os; result = os.getcwd()`
   - EXPECTED: Should fail with import error

10. Test sandbox blocks network:
    - Use rlm_exec with code: `import socket; result = "bad"`
    - EXPECTED: Should fail with import error

Report: Pass/Fail for each step. Sandbox tests MUST fail to pass!
```

---

## Expected Results

| Step | Expected | Pass Criteria |
|------|----------|---------------|
| Load src directory | Loaded with chunk count | status="loaded", chunks > 0 |
| Chunk context | Multiple chunks | chunk_count > 1 |
| Filter patterns | Filtered hits | lines > 0 in output |
| Get chunks | Chunk content returned | Content not empty |
| Batch sub-query | Results from OpenRouter | No provider error |
| findings.json | Valid JSON | Array of results |
| evidence_candidates.jsonl | Valid JSONL | At least 1 line |
| manifest.json | Skill-compliant | success=true |
| Sandbox `os` | **MUST FAIL** | Error contains "import" or "blocked" |
| Sandbox `socket` | **MUST FAIL** | Error contains "import" or "blocked" |

---

## Failure Troubleshooting

- **OpenRouter fails**: Check OPENROUTER_API_KEY is set
- **Sandbox allows os/socket**: CRITICAL security issue - fix before proceeding
- **No evidence candidates**: Check filter pattern matches actual code
- **Batch times out**: Reduce chunk count or use ollama locally

---

## Chad's Reminder

> "If either smoke test fails, stop and fix wiring/tool semantics before any budget work."
