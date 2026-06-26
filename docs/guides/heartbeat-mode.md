# Heartbeat Mode

Heartbeat mode enables autonomous, long-running agents that operate on a persistent assess→plan→act→persist→sleep cycle. This is ideal for proactive monitoring, maintenance, and continuous improvement tasks.

## Overview

A heartbeat agent runs indefinitely, following this cycle:

1. **Assess** — Evaluate the current state (check files, monitor systems, review logs)
2. **Plan** — Determine what actions to take based on the assessment
3. **Act** — Execute planned actions using available tools
4. **Persist** — Save state to disk for resumability
5. **Sleep** — Wait for the configured interval, then repeat

## Quick Start

```bash
# Run a heartbeat agent that checks every 5 minutes
ravenclaws --heartbeat --exec "Monitor the /tmp directory for new files and report changes"
```

## Configuration

```toml
[heartbeat]
interval_secs = 300        # Sleep between cycles (5 minutes)
state_file = "heartbeat-state.json"  # State persistence
max_cycles = 0             # 0 = unlimited

[llm]
provider = "openai"
model = "gpt-4o-mini"
system_prompt = "You are a vigilant system monitor. Assess, plan, and act carefully."
```

### Key Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `interval_secs` | `300` | Sleep duration between cycles |
| `state_file` | `"heartbeat-state.json"` | File for persisting state across restarts |
| `max_cycles` | `0` | Maximum cycles (0 = run forever) |

## State Persistence

Heartbeat agents automatically persist their state to disk after each cycle. This means:

- **Survives restarts** — If the agent is stopped and restarted, it resumes from the last checkpoint
- **Crash recovery** — If the process crashes mid-cycle, state up to the last persist is preserved
- **Manual inspection** — State files are human-readable JSON

### State File Contents

```json
{
  "cycle": 42,
  "last_assessment": "System healthy. 3 files changed since last check.",
  "last_plan": "Review changed files for security issues.",
  "last_action": "Scanned 3 files. No issues found.",
  "timestamp": "2026-06-02T15:30:00Z"
}
```

## Use Cases

### File System Monitoring

```bash
ravenclaws --heartbeat \
  --system-prompt "You are a file integrity monitor." \
  --exec "Monitor /etc for unauthorized changes. Alert if any file is modified."
```

### Log Analysis

```bash
ravenclaws --heartbeat \
  --system-prompt "You are a log analyst." \
  --exec "Check /var/log for error patterns every minute. Summarize new errors."
```

### Automated Maintenance

```bash
ravenclaws --heartbeat \
  --system-prompt "You are a system maintainer." \
  --exec "Clean up temporary files older than 7 days in /tmp. Report disk usage."
```

### CI/CD Health Monitor

```bash
ravenclaws --heartbeat \
  --system-prompt "You are a CI/CD health monitor." \
  --exec "Check GitHub Actions workflow status every 10 minutes. Report failures."
```

## Best Practices

1. **Set appropriate intervals** — Don't poll too frequently (respect rate limits)
2. **Use specific system prompts** — Focus the agent on its monitoring role
3. **Enable audit logging** — Track all actions for compliance
4. **Configure sandbox** — Restrict file system access to necessary paths
5. **Monitor the monitor** — Use health endpoints to verify heartbeat is running
6. **Test recovery** — Verify state persistence works by restarting the agent

## Limitations

- Heartbeat agents are single-threaded (one cycle at a time)
- State file locking is not supported for multi-process access
- Very short intervals (< 10 seconds) may cause excessive API usage
