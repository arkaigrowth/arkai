# Kimi K2 + OpenClaw/Clawdbot/Moltbot Research

**Research Date:** 2026-01-30
**Purpose:** Document Kimi K2 tool calling compatibility with OpenClaw ecosystem

---

## Executive Summary

Kimi K2 has **known tool calling issues** when used through OpenRouter and many third-party providers. The open-source version is described by MoonshotAI as "flaky for demanding workflows." While OpenClaw/Moltbot officially supports Kimi K2.5, users should expect potential tool calling failures, particularly with complex multi-turn conversations.

**Key Recommendation:** For reliable tool calling, use either:
1. Moonshot's official API (`platform.moonshot.ai`)
2. OpenRouter with provider locked to `moonshotai`, `together`, or `groq`
3. Kimi K2.5 (newer version with improved tool calling)

---

## 1. OpenClaw/Moltbot Kimi K2 Support

### Official Integration Status

OpenClaw (formerly Moltbot/Clawdbot) added Kimi K2.5 support in:
- **PR #2762** (2026-01-24): Updated Moonshot Kimi model references to kimi-k2.5
- **PR #4407** (2026-01-29): Added Kimi K2.5 to synthetic model catalog

**Sources:**
- [OpenClaw Changelog](https://github.com/openclaw/openclaw/blob/main/CHANGELOG.md)
- [OpenClaw Releases](https://github.com/openclaw/openclaw/releases)

### Configuration Example

```json
{
  "agent": {"model": {"primary": "moonshot/kimi-k2.5"}},
  "models": {
    "providers": {
      "moonshot": {
        "baseUrl": "https://api.moonshot.ai/v1",
        "apiKey": "sk-your-key",
        "api": "openai-completions",
        "models": [{
          "id": "kimi-k2.5",
          "name": "Kimi K2.5 (API)",
          "contextWindow": 262144,
          "maxTokens": 8192
        }]
      }
    }
  }
}
```

**Source:** [Apidog Guide - Kimi K2.5 with MoltBot](https://apidog.com/blog/kimi-k2-5-clawdbot/)

---

## 2. Known Tool Calling Issues

### Core Problem

Kimi K2 fails to invoke tools properly when used through OpenRouter and some other providers. Instead of returning structured `tool_calls`, it generates text responses containing JSON.

**Error Pattern:**
```
AI_NoObjectGeneratedError: No object generated: the tool was not called.
```

**Root Cause:** The problem lies in Kimi K2's interleaved thinking mode - it has difficulty calling tools inside its `<think>` tags.

### Documented Issues

| Issue | Description | Status |
|-------|-------------|--------|
| [Zed #37032](https://github.com/zed-industries/zed/discussions/37032) | Kimi K2 tool calls don't work in OpenRouter | Open |
| [MoonshotAI/Kimi-K2 #27](https://github.com/MoonshotAI/Kimi-K2/issues/27) | Claude Code facing tool calling stop | Closed |
| [MoonshotAI/Kimi-K2 #26](https://github.com/MoonshotAI/Kimi-K2/issues/26) | SGLang tool_parser invalid under multi-turn | Open |
| [MoonshotAI/Kimi-K2 #41](https://github.com/MoonshotAI/Kimi-K2/issues/41) | Complex tool calling leads to empty output | Open |
| [sst/opencode #929](https://github.com/sst/opencode/issues/929) | Error due to parsing of tool calls | Resolved |
| [OpenRouter Gist](https://gist.github.com/ben-vargas/c7c9633e6f482ea99041dd7bd90fbe09) | Detailed tool call issue documentation | N/A |

### Symptoms

- Missing `<|tool_calls_section_begin|>` token
- "null" appearing before `<|tool_call_begin|>`
- Raw tool call tokens in output (e.g., `<|tool_call_begin|>functions.Task:0<|tool_call_argument_begin|>`)
- `finish_reason: tool_calls` but `tool_calls` and `content` are `None`

---

## 3. Workarounds and Solutions

### A. Use Official Moonshot API (Recommended)

The official API includes guided-decoding and is "more stable for demanding workflows."

```python
# Using official API
import openai

client = openai.OpenAI(
    api_key="your-moonshot-key",
    base_url="https://api.moonshot.ai/v1"
)

response = client.chat.completions.create(
    model="kimi-k2.5",
    messages=[...],
    tools=[...],
    tool_choice="auto"
)
```

### B. OpenRouter Provider Locking

Create a preset that locks providers to known-working ones:

```json
{
  "extra_body": {
    "provider": {
      "only": ["moonshotai", "together", "groq"]
    }
  },
  "max_tokens": 0  // Let OpenRouter handle automatically
}
```

**Avoid:** Baseten, DeepInfra (fp4/fp8 quantization issues), Novita (type validation errors)

### C. Manual Tool Call Parsing

If your service doesn't support built-in parsing, parse the raw output:

```python
import re

def parse_kimi_tool_calls(text):
    """Parse Kimi K2's native tool call format."""
    pattern = r'<\|tool_call_begin\|>([^<]+)<\|tool_call_argument_begin\|>(.+?)<\|tool_call_end\|>'
    matches = re.findall(pattern, text, re.DOTALL)

    tool_calls = []
    for tool_id, arguments in matches:
        # tool_id format: functions.{func_name}:{idx}
        func_name = tool_id.split('.')[1].split(':')[0]
        tool_calls.append({
            "id": tool_id,
            "function": {
                "name": func_name,
                "arguments": arguments.strip()
            }
        })
    return tool_calls
```

### D. Upgrade to Kimi K2.5

K2.5 includes improved tool calling with:
- Better multimodal support
- Up to 100 sub-agents for parallel tool execution
- 256K token context
- Continued pretraining on 15T tokens

### E. vLLM Configuration

For self-hosted deployments:

```bash
vllm serve moonshotai/Kimi-K2-Instruct \
    --tensor-parallel-size 8 \
    --enable-auto-tool-choice \
    --tool-call-parser kimi_k2 \
    --reasoning-parser kimi_k2 \
    --trust-remote-code
```

**Important:** Use models with updated chat templates:
- Kimi-K2-0905: commit `94a4053`
- Kimi-K2: commit `0102674`

---

## 4. Provider Performance Comparison

### K2-Vendor-Verifier Results

MoonshotAI maintains a tool for verifying provider accuracy: [K2-Vendor-Verifier](https://github.com/MoonshotAI/K2-Vendor-Verifier)

**Key Metrics:**
- `tool_call_f1`: Determines if model deployment is correct
- `schema_accuracy`: Measures JSON payload schema compliance

### Provider Rankings (General Performance)

| Provider | Speed | Quality | Tool Calling Notes |
|----------|-------|---------|-------------------|
| Moonshot AI | 10 tok/s | 9/10 | Most reliable, official API |
| Groq | 170+ tok/s | 8.5-9.5 | Fastest, good for speed |
| Together | 40 tok/s | 8-9 | Stable, balanced |
| DeepInfra | 60-73 tok/s | 8.5-10 | fp4 quantization issues with tools |
| Novita | Variable | 8.5-9 | Unstable, `type` field errors |

**Sources:**
- [16x.engineer Provider Evaluation](https://eval.16x.engineer/blog/kimi-k2-provider-evaluation-results)
- [K2-Vendor-Verifier GitHub](https://github.com/MoonshotAI/K2-Vendor-Verifier)

---

## 5. vLLM Tool Calling Deep Dive

### Initial Problems (October 2025)

Per the [vLLM blog post](https://blog.vllm.ai/2025/10/28/Kimi-K2-Accuracy.html), initial tool calling had ~17% success rate (218/1,286 calls).

### Root Causes Identified

1. **Missing `add_generation_prompt`**: vLLM couldn't detect this required parameter
2. **Empty `content` field processing**: Empty strings converted to list-of-dicts, breaking Jinja templates
3. **Strict tool-call ID parser**: Expected `functions.func_name:idx` format exactly

### Fixes Applied

- **vLLM PR #27622**: Whitelists chat-template parameters
- **vLLM PR #27565**: Improves parser robustness
- **vLLM PR #28543**: Prevents special token leakage in streaming mode

**Post-fix success rate:** ~76% (1,007/1,286 calls) - 4.4x improvement

### Remaining Limitation

vLLM lacks constrained decoding ("Enforcer"), allowing hallucinated tool calls for undeclared tools.

---

## 6. Kimi K2 Tool Call Format Reference

### Native Format

```
<|tool_calls_section_begin|>
<|tool_call_begin|>functions.get_weather:0<|tool_call_argument_begin|>{"city": "Beijing"}<|tool_call_end|>
<|tool_calls_section_end|>
```

### API Response Format (OpenAI-compatible)

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": null,
      "tool_calls": [{
        "id": "functions.get_weather:0",
        "type": "function",
        "function": {
          "name": "get_weather",
          "arguments": "{\"city\": \"Beijing\"}"
        }
      }]
    },
    "finish_reason": "tool_calls"
  }]
}
```

---

## 7. Recommendations for Claudia/arkai Integration

### Short-term

1. **Use Moonshot official API** for tool calling reliability
2. **Set max_tokens explicitly** (8192 or higher)
3. **Use temperature 0.6** (recommended by Moonshot)
4. **Normalize tool_call_ids** to `functions.{name}:{idx}` format

### Medium-term

1. **Upgrade to Kimi K2.5** when stable
2. **Implement fallback parsing** for raw tool call tokens
3. **Monitor K2-Vendor-Verifier** for provider updates

### Configuration for OpenRouter

```json
{
  "model": "moonshotai/kimi-k2.5",
  "extra_body": {
    "provider": {
      "only": ["moonshotai", "together", "groq"]
    }
  }
}
```

---

## 8. Community Resources

- **Moonshot Discord:** discord.com/invite/TYU2fdJykW
- **Reddit:** r/kimi
- **GitHub:** [MoonshotAI/Kimi-K2](https://github.com/MoonshotAI/Kimi-K2)
- **Official Docs:** [platform.moonshot.ai](https://platform.moonshot.ai/docs)
- **K2 Vendor Verifier:** [GitHub](https://github.com/MoonshotAI/K2-Vendor-Verifier)

---

## 9. Sources Referenced

### GitHub Issues/Discussions
- [Zed #37032 - Kimi K2 tool calls don't work in openrouter](https://github.com/zed-industries/zed/discussions/37032)
- [MoonshotAI/Kimi-K2 #27 - Claude Code tool calling stop](https://github.com/MoonshotAI/Kimi-K2/issues/27)
- [MoonshotAI/Kimi-K2 #26 - SGLang tool_parser invalid](https://github.com/MoonshotAI/Kimi-K2/issues/26)
- [MoonshotAI/Kimi-K2 #41 - Complex tool calling issues](https://github.com/MoonshotAI/Kimi-K2/issues/41)
- [sst/opencode #929 - Tool call parsing error](https://github.com/sst/opencode/issues/929)
- [sglang #12932 - Optional parameters dropped](https://github.com/sgl-project/sglang/issues/12932)
- [ikawrakow/ik_llama.cpp #865 - Tool calling segfault](https://github.com/ikawrakow/ik_llama.cpp/issues/865)

### Technical Documentation
- [vLLM Blog - Debugging Kimi K2 Tool-Calling](https://blog.vllm.ai/2025/10/28/Kimi-K2-Accuracy.html)
- [Kimi K2 Tool Call Guidance](https://github.com/MoonshotAI/Kimi-K2/blob/main/docs/tool_call_guidance.md)
- [vLLM Kimi-K2 Recipe](https://docs.vllm.ai/projects/recipes/en/latest/moonshotai/Kimi-K2.html)
- [Moonshot Platform Docs](https://platform.moonshot.ai/docs/guide/use-kimi-api-to-complete-tool-calls)

### Configuration Guides
- [Apidog - Kimi K2.5 with MoltBot](https://apidog.com/blog/kimi-k2-5-clawdbot/)
- [OpenRouter Kimi K2.5](https://openrouter.ai/moonshotai/kimi-k2.5)
- [claude-code-router #245](https://github.com/musistudio/claude-code-router/issues/245)

### Tool Call Issue Gist
- [ben-vargas/openrouter-kimi-k2-tool-call-issue.md](https://gist.github.com/ben-vargas/c7c9633e6f482ea99041dd7bd90fbe09)

---

**Last Updated:** 2026-01-30
