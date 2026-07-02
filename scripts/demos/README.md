# RavenClaws Demo Suite

A collection of focused asciinema terminal demos, each showcasing a specific
set of RavenClaws features. Designed for different audiences and use cases.

## Demos

| # | Script | Focus | Audience | Duration |
|---|---|---|---|---|
| 1 | `demo-quickstart.sh` | Version, binary profile, config, exec mode | Users | ~30s |
| 2 | `demo-architecture.sh` | Source modules, test suite, patterns, healing, load | Developers | ~45s |
| 3 | `demo-server-mcp.sh` | HTTP server, MCP server, MCP SSE server | Operators | ~40s |
| 4 | `demo-resilience.sh` | Self-healing engine, circuit breakers, graceful degradation | SREs | ~35s |
| 5 | `demo-deployment.sh` | Docker, K8s, website, verification suite | DevOps | ~40s |

## Recording

```bash
# Record all demos
for demo in demo-quickstart demo-architecture demo-server-mcp demo-resilience demo-deployment; do
  asciinema rec --title "RavenClaws v1.2.0 — ${demo#demo-}" \
    --command "./scripts/demos/$demo.sh" \
    --overwrite "demos/$demo.cast"
done

# Upload all
for f in demos/*.cast; do
  asciinema upload "$f"
done
```

## Running Without Recording

```bash
./scripts/demos/demo-quickstart.sh
./scripts/demos/demo-architecture.sh
# etc.
```
