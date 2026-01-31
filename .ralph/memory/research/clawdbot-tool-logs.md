# Clawdbot Gateway Tool Logs Investigation

**Date:** 2026-01-30
**Clawdbot Version:** 2026.1.24-3
**Model:** openrouter/moonshotai/kimi-k2-0905

---

## Summary

Tool functionality is **working** but there are several configuration warnings and issues that should be addressed.

---

## Key Findings

### 1. Tools ARE Working

Tools are being successfully called and executed:

| Tool | Calls Today |
|------|-------------|
| read | 36 |
| web_search | 18 |
| exec | 12 |
| write | 10 |

**Evidence from logs:**
```
embedded run tool start: runId=... tool=read toolCallId=read:15
embedded run tool end: runId=... tool=read toolCallId=read:15
embedded run tool start: runId=... tool=exec toolCallId=exec:26
embedded run tool end: runId=... tool=exec toolCallId=exec:26
```

### 2. Unknown Tool Entries Warning (Non-Critical)

**Warning (recurring on every run):**
```
tools.allow allowlist contains unknown entries (glob, search, grep).
These entries won't match any tool unless the plugin is enabled.
```

**Cause:** The `tools.allow` list in `~/.clawdbot/clawdbot.json` contains:
```json
"allow": ["read", "write", "edit", "glob", "search", "grep", "web_search", "web_fetch", "exec"]
```

The entries `glob`, `search`, and `grep` are not valid tool names in clawdbot. They might be confused with Claude Code tool names.

**Fix:** Remove `glob`, `search`, and `grep` from the allowlist since they don't exist in clawdbot.

### 3. Exec Tool Configuration Errors (Historical)

Several config reload attempts failed due to invalid `tools.exec` configuration:

```
Invalid config at /home/clawdbot/.clawdbot/clawdbot.json:
- tools.exec: Unrecognized keys: "mode", "allowlist"
- tools.exec: Unrecognized keys: "allowedCommands", "blockedCommands"
```

**Current State:** Config was fixed - `tools.exec` is now an empty object `{}` which is valid.

### 4. Legacy Bash Tool Migration

```
tools.bash: tools.bash was removed; use tools.exec instead (auto-migrated on load).
```

This is just informational - the migration is automatic.

### 5. File Access Errors (Expected)

Some tool failures are due to missing files (expected behavior):
```
read failed: ENOENT: no such file or directory, access '/home/clawdbot/clawd/arkai'
read failed: ENOENT: no such file or directory, access '/home/clawdbot/clawd/arkai/roadmap.md'
exec failed: ls: cannot access '/home/clawdbot/clawd/arkai/': No such file or directory
```

This is normal - the model tried to access files that don't exist.

### 6. Model-Specific Tool Compatibility

The Kimi K2 model via OpenRouter is successfully using tools:
- Tool calls use `functions.` prefix in toolCallId (e.g., `functions.bash:21`, `functions.exec:25`)
- Tools are being mapped correctly: `functions.bash` -> `exec` tool
- No tool format errors or API rejections observed

---

## Current Tool Configuration

From `~/.clawdbot/clawdbot.json`:

```json
{
  "tools": {
    "allow": ["read", "write", "edit", "glob", "search", "grep", "web_search", "web_fetch", "exec"],
    "deny": ["process", "browser", "canvas", "cron"],
    "web": {
      "search": {
        "enabled": true,
        "provider": "perplexity",
        "perplexity": {
          "baseUrl": "https://openrouter.ai/api/v1",
          "model": "perplexity/sonar-pro"
        }
      },
      "fetch": { "enabled": true }
    },
    "media": {
      "audio": {
        "enabled": true,
        "models": [{ "provider": "openai", "model": "whisper-1" }]
      }
    },
    "exec": {}
  }
}
```

---

## Recommendations

### High Priority

1. **Clean up `tools.allow` list** - Remove invalid entries:
   ```json
   "allow": ["read", "write", "edit", "web_search", "web_fetch", "exec"]
   ```
   This will stop the recurring "unknown entries" warnings.

### Low Priority

2. **Create symlink for arkai access** - If Claudia needs to read arkai files:
   ```bash
   ln -s ~/arkai ~/clawd/arkai
   ```

3. **Consider enabling more plugins** - Only 2/28 plugins are currently loaded:
   - Memory (Core) - loaded
   - Telegram - loaded (via channels config)

---

## Available Tools Summary

**Enabled in config:**
- read (working)
- write (working)
- edit (in allowlist)
- web_search (working)
- web_fetch (in allowlist)
- exec (working)

**Denied:**
- process
- browser
- canvas
- cron

---

## Raw Log Locations

- Main log: `/tmp/clawdbot/clawdbot-2026-01-30.log`
- Gateway log: `~/gateway.log` (abbreviated version)

---

## Conclusion

Tools are functioning correctly. The main issue is cosmetic - the `tools.allow` list contains some Claude Code tool names that don't exist in clawdbot (`glob`, `search`, `grep`). This causes warning messages but doesn't affect functionality.

The exec tool (bash commands) is working and properly mapping from the model's `functions.bash` calls to clawdbot's `exec` tool.
