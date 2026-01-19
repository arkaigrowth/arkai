# RLM Smoke Test A: Tiny + Deterministic

> **Purpose**: Prove MCP wiring and basic tool call flow
> **Paste this into Claude Code after restart**

---

## Test Prompt

```
Run RLM Smoke Test A (tiny + deterministic):

1. Load the arkai README.md as context:
   - Use rlm_load_context with name="readme" and the README content

2. Filter for specific patterns:
   - Use rlm_filter_context with pattern="ingest|library|pipeline"
   - Output name: "readme_keywords"

3. Execute safe Python code:
   - Use rlm_exec to count matches per line
   - Code: `result = len(sys.stdin.read().split('\n'))`
   - Context: "readme_keywords"

4. Output results as manifest.json with this structure:
   {
     "skill": "rlm-smoke-test-a",
     "success": true,
     "artifacts": [
       {"name": "filter_output.txt", "type": "text", "lines": <count>}
     ],
     "result": {"test": "smoke_a", "status": "passed", "filter_matches": <count>}
   }

Report: Pass/Fail for each step with any error messages.
```

---

## Expected Results

| Step | Expected | Pass Criteria |
|------|----------|---------------|
| rlm_load_context | Returns metadata (length, lines, hash) | No error, status="loaded" |
| rlm_filter_context | Returns filtered context | status="filtered", lines > 0 |
| rlm_exec | Returns line count | result is integer > 0 |
| manifest.json | Valid JSON | success=true |

---

## Failure Troubleshooting

- **"Unknown provider"**: Check MCP server loaded (`claude mcp list`)
- **"Context not found"**: rlm_load_context must be called first
- **"Exec timeout"**: Code took >30s, simplify
- **"Import blocked"**: Sandbox working correctly (expected for unsafe imports)
