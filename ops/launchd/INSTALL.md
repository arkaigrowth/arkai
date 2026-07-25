# Voice Poller Launchd Install

Run from the repo root.

1. Install the current binary:

   ```bash
   cargo install --path .
   ```

   Re-grant Full Disk Access to `~/.cargo/bin/arkai`. TCC binds the inode, so every `cargo install` resets the grant. Only the watcher needs FDA.

2. Run one manual scan from an interactive terminal:

   ```bash
   arkai voice scan
   ```

   This surfaces the LuLu re-prompt for the new binary hash so a human can answer it. Under launchd it would silently time out.

3. Back up the queue before enabling automation:

   ```bash
   cp ~/.arkai/voice_queue.jsonl{,.bak-$(date +%s)}
   ```

4. Healthcheck is already provisioned out-of-band on 2026-07-07 on the local healthchecks instance: `voice-watcher (arkai voice scan)`, timeout 24h / grace 6h. The ping URL is written to `~/.arkai/hc_ping_url`. The check starts in `new` status and arms itself on the first real scan ping. Server-side re-alerts are the still-broken heartbeat; there is no local alert-state machine.

5. Verify Telegram credentials:

   ```bash
   stat -f '%OLp %N' ~/.arkai/telegram_token
   ```

   The mode must be `600`, and the file must contain the dedicated `@arkai_voice_bot` credentials.

6. Render and load both LaunchAgents. launchd resolves neither `~` nor `$HOME` inside a plist, so the checked-in files are templates carrying a `__HOME__` placeholder that you substitute at install time:

   ```bash
   for agent in voice-watcher voice-processor; do
     sed "s|__HOME__|$HOME|g" "ops/launchd/com.local.$agent.plist.template" \
       > "$HOME/Library/LaunchAgents/com.local.$agent.plist"
     chmod 644 "$HOME/Library/LaunchAgents/com.local.$agent.plist"
   done
   launchctl load ~/Library/LaunchAgents/com.local.voice-{watcher,processor}.plist
   ```

   Watch both logs through one full cycle:

   ```bash
   tail -f ~/Library/Logs/voice-watcher.log ~/Library/Logs/voice-processor.log
   ```

   Confirm a test file flows detect -> copy -> awaiting -> ping -> approve (human) -> local route -> vault.

7. First-run expectation: the ~142-item historical backlog stays awaiting, which is correct. Approving a memo whose source has since been deleted fails cleanly with `source unreadable`.
