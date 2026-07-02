#!/usr/bin/env bash
# RavenClaws asciinema demo script (legacy — use scripts/demos/ suite instead)
#
# This monolithic demo is preserved for backwards compatibility.
# For focused demos, use the scripts/demos/ suite:
#
#   ./scripts/demos/demo-quickstart.sh     # Version, config, exec mode
#   ./scripts/demos/demo-architecture.sh   # Modules, tests, patterns
#   ./scripts/demos/demo-server-mcp.sh     # HTTP, MCP, SSE servers
#   ./scripts/demos/demo-resilience.sh     # Self-healing, circuit breakers
#   ./scripts/demos/demo-deployment.sh     # Docker, K8s, website
#
# Record:
#   asciinema rec --title "RavenClaws v1.2.0 Demo" \
#     --command "./scripts/demo.sh" \
#     --overwrite demo.cast

echo "⚠️  This monolithic demo is deprecated."
echo "   Use the focused demo suite instead:"
echo "   ./scripts/demos/demo-quickstart.sh"
echo "   ./scripts/demos/demo-architecture.sh"
echo "   ./scripts/demos/demo-server-mcp.sh"
echo "   ./scripts/demos/demo-resilience.sh"
echo "   ./scripts/demos/demo-deployment.sh"
echo
echo "   See docs/guides/demo.md for details."
exit 0
