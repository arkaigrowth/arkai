# OpenRouter Tool Calling Research

> Research Date: 2026-01-30
> Purpose: Understand OpenRouter's tool calling implementation and model compatibility

---

## Executive Summary

OpenRouter standardizes tool calling across 400+ models using an OpenAI-compatible interface. While most Claude models work seamlessly, Gemini 3 models have significant compatibility issues requiring special handling of `thought_signature` data. Some models (like Kimi K2) advertise tool support but don't properly invoke tools.

---

## 1. How OpenRouter Handles Tool Calls

### Unified Interface

OpenRouter standardizes the tool calling interface across models and providers, making it easy to integrate external tools with any supported model. The process follows a three-step pattern:

1. **Initial Request**: Send user message with tool definitions
2. **Tool Execution**: Client executes the tool locally based on model's suggestion
3. **Follow-up Request**: Return tool results to the model for final response

**Key principle**: Models don't execute tools directly - they suggest which tool to invoke, and the client handles actual execution.

### Request Format

```json
{
  "model": "anthropic/claude-sonnet-4",
  "messages": [...],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get weather for a location",
        "parameters": {
          "type": "object",
          "properties": {
            "location": { "type": "string" }
          },
          "required": ["location"]
        }
      }
    }
  ],
  "tool_choice": "auto"
}
```

### Response Format

When a model wants to call a tool:

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "tool_calls": [
        {
          "id": "call_abc123",
          "type": "function",
          "function": {
            "name": "get_weather",
            "arguments": "{\"location\": \"San Francisco\"}"
          }
        }
      ]
    },
    "finish_reason": "tool_calls"
  }]
}
```

Tool results are sent back as:

```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "72F, sunny"
}
```

---

## 2. API Parameters for Tool Calling

### Core Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `tools` | array | Tool definitions following OpenAI function calling schema |
| `tool_choice` | string/object | Controls tool invocation: `"auto"`, `"none"`, `"required"`, or specific tool |
| `parallel_tool_calls` | boolean | Whether to allow multiple simultaneous tool calls (default: true) |

### tool_choice Options

- `"auto"` (default): Model decides whether to use tools
- `"none"`: Disables tool usage entirely
- `"required"`: Model must call one or more tools
- `{"type": "function", "function": {"name": "my_function"}}`: Forces specific tool

### Provider-Specific Parameters

OpenRouter supports passing provider-specific parameters via `extra_body`:

```python
response = client.chat.completions.create(
    model="anthropic/claude-sonnet-4",
    messages=[...],
    tools=[...],
    extra_body={
        "provider": {
            "anthropic": {
                # Anthropic-specific params
            }
        }
    }
)
```

---

## 3. Model-Specific Configurations

### Claude Models (Anthropic)

**Status**: Works well with minimal configuration

- Tool IDs use `toolu_...` format (Anthropic style) even through OpenRouter
- Supports parallel tool calls
- Extended thinking/reasoning blocks supported
- Native tool use works through OpenRouter's "Anthropic Skin" endpoint

**Configuration Notes**:
- For Claude Code integration, use `ANTHROPIC_BASE_URL=https://openrouter.ai/api`
- OpenRouter's Anthropic-compatible endpoint handles "Thinking" blocks and native tool use

**Known Working Models**:
- `anthropic/claude-sonnet-4`
- `anthropic/claude-sonnet-4-20250514`
- `anthropic/claude-opus-4`
- `anthropic/claude-3.5-sonnet`
- `anthropic/claude-3.5-haiku`

### Gemini Models (Google)

**Status**: SIGNIFICANT ISSUES with Gemini 3 models

#### The `thought_signature` Problem

Gemini 3 models require a `thought_signature` field in function call requests:

```
Error: "Function call is missing a thought_signature in functionCall parts.
This is required for tools to work correctly, and missing thought_signature
may lead to degraded model performance."
```

**Root Cause**: When Gemini 3 returns a tool call, it includes `reasoning_details` with a `thought_signature`. This must be preserved and sent back in the follow-up request. Many clients strip this data.

**Affected Models**:
- `google/gemini-3-pro-preview`
- `google/gemini-3-flash-preview`

**Working Models** (no thought_signature requirement):
- `google/gemini-2.5-flash`
- `google/gemini-2.5-pro`

**Workaround**: Preserve `reasoning_details` from model responses and include them in subsequent requests:

```json
{
  "messages": [
    ...,
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [...],
      "reasoning_details": [
        {
          "type": "reasoning.text",
          "text": "...",
          "signature": "..." // Must preserve this
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "...",
      "content": "..."
    }
  ]
}
```

**Additional Issues**:
- Native function calling fails with multiple tools having long descriptions
- Some Gemini models "hallucinate" tool calls (return text describing the call instead of structured output)

### DeepSeek Models

**Status**: Generally works, with some caveats

- DeepSeek R1 0528 supports native tool calling on OpenRouter (announced June 2025)
- DeepSeek V3.1 and V3.2 have improved tool use
- Free tier models (`:free` suffix) may have issues with tool detection

**Known Issues**:
- `:free` suffix models may fail with "No endpoints found that support tool use"
- Some applications incorrectly parse the `:free` suffix

### Kimi K2 (Moonshot)

**Status**: BROKEN - Does not properly invoke tools

**Problem**: Despite advertising "Tools" and "Tool Choice" support, Kimi K2 returns plain text responses containing JSON instead of structured tool calls:

```json
// Expected:
{
  "tool_calls": [...]
}

// Actual:
{
  "content": "Here are the tasks:\n\n```json\n{...}```"
}
```

**Workaround**: Parse JSON from text responses manually, or use a different model.

### OpenAI Models

**Status**: Works well

- Standard OpenAI tool calling format
- Supports `parallel_tool_calls`
- GPT-4o series fully compatible

---

## 4. Format Differences Between Providers

### Tool Call ID Formats

| Provider | Format | Example |
|----------|--------|---------|
| Claude | `toolu_` prefix | `toolu_01abc123` |
| OpenAI | `call_` prefix | `call_abc123xyz` |
| Gemini | Varies | May use different format |

**Note**: When using Claude through OpenRouter, you get `toolu_...` IDs even with the OpenAI-compatible API.

### Reasoning/Thinking Blocks

| Provider | Parameter | Notes |
|----------|-----------|-------|
| Anthropic | `reasoning.max_tokens` | Minimum 1024 tokens |
| Gemini | `reasoning.effort` | Maps to `thinkingLevel` internally |
| OpenAI | `reasoning.effort` | Levels: `none` to `xhigh` |

### API Endpoint Compatibility

OpenRouter offers two API styles:

1. **OpenAI-compatible** (`/api/v1/chat/completions`):
   - Standard for most use cases
   - Some provider-specific features may not translate

2. **Anthropic-compatible** (`/api` with Anthropic SDK):
   - "Anthropic Skin" - behaves exactly like Anthropic API
   - Supports Thinking blocks and native tool use
   - Better for Claude Code integration

---

## 5. Known Compatibility Issues

### Critical Issues

| Model | Issue | Severity | Status |
|-------|-------|----------|--------|
| Gemini 3 Pro/Flash | Missing thought_signature | BLOCKING | Partially fixed (direct API only) |
| Kimi K2 | Doesn't invoke tools properly | BROKEN | No fix |
| Gemini 2.5 Flash | Hallucinated tool calls | Intermittent | Ongoing |

### Provider-Specific Quirks

1. **Gemini via OpenRouter**: Fixes for direct Gemini API don't apply to OpenRouter path
2. **Free Tier Models**: May not support tools despite base model supporting them
3. **Long Tool Descriptions**: Can cause failures with some models (especially Gemini)
4. **Functions Without Arguments**: Some models fail on no-argument functions

### Streaming Considerations

- Monitor for `tool_calls` in delta updates
- Check `finish_reason: "tool_calls"` to detect tool call completion
- Some providers have inconsistent streaming behavior with tool calls

---

## 6. Best Practices

### Model Selection

1. **For reliability**: Use Claude models (best tool calling support)
2. **For cost**: Use DeepSeek V3.x or GPT-4o-mini (balance of cost/capability)
3. **Avoid for tools**: Kimi K2, Gemini 3 (via OpenRouter), free tier models

### Implementation Recommendations

1. **Preserve all response fields**: Don't strip `reasoning_details`, `extra_content`, or unknown fields
2. **Include tools in all requests**: The `tools` parameter must appear in follow-up requests for schema validation
3. **Parse tool arguments**: Arguments come as JSON strings requiring parsing
4. **Implement retry logic**: Tool calls may fail intermittently
5. **Set iteration limits**: Prevent infinite loops in agentic implementations

### Testing Strategy

```
1. Test tool calling with each target model
2. Test parallel tool calls if using that feature
3. Test streaming + tool calls together
4. Test tool results with large responses
5. Test error handling when model fails to invoke tools
```

### Finding Compatible Models

Filter models by tool support: `https://openrouter.ai/models?supported_parameters=tools`

---

## 7. Quick Reference

### Working Tool Calling Models (Recommended)

- `anthropic/claude-sonnet-4` - Best overall
- `anthropic/claude-3.5-sonnet` - Good balance
- `openai/gpt-4o` - Reliable
- `deepseek/deepseek-chat` (V3.x) - Cost effective

### Models to Avoid for Tool Calling

- `moonshotai/kimi-k2` - Broken tool invocation
- `google/gemini-3-*` (via OpenRouter) - thought_signature issues
- Any model with `:free` suffix - May lack tool support

### Diagnostic Checklist

When tool calling fails:

1. Check if model supports tools: `?supported_parameters=tools`
2. Check for `finish_reason: "tool_calls"` in response
3. Verify tool schema is valid JSON Schema
4. Check if `reasoning_details` needs preservation (Gemini)
5. Try without parallel tool calls
6. Check for hallucinated responses (text instead of tool_calls)

---

## Sources

- [OpenRouter Tool Calling Documentation](https://openrouter.ai/docs/guides/features/tool-calling)
- [OpenRouter API Parameters](https://openrouter.ai/docs/api/reference/parameters)
- [OpenRouter Reasoning Tokens](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens)
- [OpenRouter Claude Code Integration](https://openrouter.ai/docs/guides/guides/claude-code-integration)
- [Tool Calling Models Collection](https://openrouter.ai/collections/tool-calling-models)
- [GitHub: Roo-Code Gemini 3 Issue #10307](https://github.com/RooCodeInc/Roo-Code/issues/10307)
- [GitHub: Continue Gemini 3 Issue #8980](https://github.com/continuedev/continue/issues/8980)
- [GitHub: claude-code-router Gemini Issue #1024](https://github.com/musistudio/claude-code-router/issues/1024)
- [GitHub: Kimi K2 Tool Call Issue](https://gist.github.com/ben-vargas/c7c9633e6f482ea99041dd7bd90fbe09)
- [OpenRouter Tool Calling Demo Repository](https://github.com/OpenRouterTeam/tool-calling)
