# What is "Fabric's LLM"? — Clarification

## The Confusion

When I said "use fabric's LLM", this was **imprecise language**. Let me clarify:

**Fabric does NOT have its own LLM.**

Fabric is a CLI tool that **calls YOUR configured LLM provider**.

---

## How Fabric Uses LLMs

### Your Current Configuration

From `~/.config/fabric/.env`:
```bash
DEFAULT_VENDOR=OpenRouter
DEFAULT_MODEL=anthropic/claude-sonnet-4
OPENROUTER_API_BASE_URL=https://openrouter.ai/api/v1
```

### What This Means

When you run:
```bash
echo "some text" | fabric -p extract_wisdom
```

Fabric does:
1. Loads the `extract_wisdom` pattern (system prompt)
2. Combines it with your input text
3. Sends it to **OpenRouter** (your configured vendor)
4. OpenRouter routes it to **Claude Sonnet 4** (your configured model)
5. Returns the response

**"Fabric's LLM" = "The LLM YOU configured fabric to use"**

---

## What is Fabric Mill / --serve?

Fabric has a server mode:
```bash
fabric --serve
```

This starts an HTTP API server that:
- Exposes fabric patterns as REST endpoints
- Allows other tools to call fabric over HTTP instead of subprocess
- Enables web UIs to interact with fabric

### Example:
```bash
# Start mill server
fabric --serve --port 8080

# Call pattern via HTTP
curl -X POST http://localhost:8080/pattern/extract_wisdom \
  -d "Your input text here"
```

**Why it exists:** For building web apps or services that use fabric patterns without subprocess spawning.

**Do you need it?** Probably not. arkai uses subprocess mode (calling `fabric -p pattern` directly), which works fine for CLI use.

---

## Available LLM Providers in Fabric

Fabric supports multiple providers via `--setup`:

| Provider | Config Key | Notes |
|----------|------------|-------|
| OpenRouter | `OPENROUTER_API_KEY` | Aggregator, many models |
| OpenAI | `OPENAI_API_KEY` | GPT-4, etc. |
| Anthropic | `ANTHROPIC_API_KEY` | Claude models |
| Ollama | `OLLAMA_HOST` | Local models, free |
| Azure OpenAI | `AZURE_*` | Enterprise |
| Google AI | `GOOGLE_AI_API_KEY` | Gemini |

You can switch models with:
```bash
fabric -p extract_wisdom --model gpt-4o
fabric -p extract_wisdom --model ollama/llama3
```

---

## Summary

| Term | What It Actually Means |
|------|------------------------|
| "Fabric's LLM" | The LLM YOU configured (OpenRouter/Claude Sonnet 4) |
| Fabric Mill | HTTP server mode for fabric (optional) |
| `fabric -p pattern` | Runs pattern through your configured LLM |
| `fabric --setup` | Wizard to configure your LLM providers |

**Bottom line:** Fabric is a prompt router, not an LLM. It sends YOUR prompts to YOUR LLM provider.
