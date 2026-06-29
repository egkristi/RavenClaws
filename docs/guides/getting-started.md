# Getting Started with RavenClaws

This guide walks you through your first RavenClaws agent, from installation to running a multi-step task.

## Prerequisites

- **Rust 1.86+** (for building from source)
- **Docker** (optional, for containerized deployment)
- **An LLM provider** — one of:
  - [LiteLLM](https://litellm.ai/) (recommended for local development)
  - [OpenAI](https://platform.openai.com/)
  - [OpenRouter](https://openrouter.ai/)
  - [Ollama](https://ollama.ai/) (fully local, no API key needed)
  - [Anthropic](https://console.anthropic.com/)

## Installation

### Option 1: Install via Cargo (recommended)

```bash
cargo install ravenclaws
```

### Option 2: Build from source

```bash
git clone https://github.com/egkristi/RavenClaws.git
cd RavenClaws
cargo build --release
# Binary is at target/release/ravenclaws
```

### Option 3: Docker

```bash
docker pull ghcr.io/egkristi/ravenclaws:latest
```

## Quick Start: Your First Agent

### 1. Set up your LLM provider

Set the required environment variables:

```bash
# For LiteLLM (default):
export LITELLM_API_KEY="your-key"
export LITELLM_ENDPOINT="http://localhost:4000"

# Or for OpenAI:
export OPENAI_API_KEY="sk-..."
export RAVENCLAWS__LLM__PROVIDER="openai"

# Or for Ollama (fully local):
export RAVENCLAWS__LLM__PROVIDER="ollama"
export RAVENCLAWS__LLM__ENDPOINT="http://localhost:11434"
export RAVENCLAWS__LLM__MODEL="llama3.1"
```

### 2. Run a one-shot task

```bash
ravenclaws --exec "What is the capital of France?"
```

You should see a response like: `The capital of France is Paris.`

### 3. Start an interactive REPL

```bash
ravenclaws --repl
```

This starts an interactive session where you can have a back-and-forth conversation:

```
╭─ RavenClaws REPL ─────────────────────────────╮
│ Type /exit to quit, /reset to clear history    │
╰────────────────────────────────────────────────╯
You: Summarize the key features of Rust
RavenClaws: Rust is a systems programming language...
You: /exit
Goodbye!
```

### 4. Run a multi-step task with tools

```bash
ravenclaws --exec "Create a file called hello.txt with the text 'Hello, RavenClaws!' and then read it back"
```

This uses the built-in `write_file` and `read_file` tools to complete the task.

## Understanding `--exec` Mode

The `--exec` flag runs a one-shot task and prints the response to stdout. It's
designed for scripting, automation, and CI/CD pipelines.

### How `--exec` Works

1. RavenClaws sends your prompt to the configured LLM
2. The agent loop runs: the LLM can plan, call tools, and produce a final answer
3. When the LLM signals completion (via `FINAL:` marker or tool call resolution),
   the response is printed to stdout
4. The process exits with code 0

### Model Compatibility

`--exec` works with all models, but the behavior depends on the model's output format:

| Model Behavior | What Happens | Example Models |
|---|---|---|
| **Uses `FINAL:` marker** | Agent loop detects `FINAL:` and prints the response | GPT-4o, Claude 3.5, Llama 3.1 (with system prompt) |
| **Emits structured tool calls** | Agent loop executes tools, then prints final response | GPT-4o, Claude 3 (tool use mode) |
| **No `FINAL:`, no tool calls** | Agent loop needs `--no-final-required` to complete | DeepSeek V4 Pro, some fine-tuned models |

### Using `--no-final-required`

If your model doesn't emit `FINAL:` or structured tool calls, add the
`--no-final-required` flag:

```bash
ravenclaws --exec "Say hello" --no-final-required
```

This tells the agent loop to treat any non-tool-call response as the final answer.

### Using `--verbose` for Debugging

To see the full LLM response content (including intermediate thoughts and tool calls):

```bash
ravenclaws --exec "Summarize the repo" --verbose
```

This sets the log level to `debug`, showing:
- LLM response content (first 500 chars)
- Tool call arguments and results
- Agent loop iteration progress

### Exit Codes

| Exit Code | Meaning |
|---|---|
| 0 | Success — task completed and response printed |
| 1 | Error — configuration error, LLM unreachable, or task failed |

## Configuration

RavenClaws can be configured via three layers (each overrides the previous):

1. **Config file** (`ravenclaws.toml`)
2. **Environment variables** (prefixed with `RAVENCLAWS__`)
3. **CLI flags**

### Minimal config file

```toml
[llm]
provider = "litellm"
endpoint = "http://localhost:4000"
api_key = "your-key"
model = "gpt-4o-mini"
```

### Full config file

See the [Configuration Reference](./configuration.md) for all available options.

## Kubernetes Deployment

RavenClaws includes a production-ready Kubernetes deployment manifest at
[`k8s/deployment.yaml`](https://github.com/egkristi/RavenClaws/blob/master/k8s/deployment.yaml).

### Prerequisites

- A Kubernetes cluster (v1.24+)
- `kubectl` configured with cluster access
- Container registry access (default: `ghcr.io/egkristi/ravenclaws`)

### Quick Deploy

```bash
kubectl apply -f k8s/deployment.yaml
```

This creates:
- A `ravenclaws` namespace
- A `ravenclaws-secrets` Secret (update the values first!)
- A `ravenclaws-config` ConfigMap
- A Deployment with 1 replica
- A ServiceAccount, Role, and RoleBinding for RBAC
- A NetworkPolicy for network isolation

### Required Secrets

The deployment expects a Secret named `ravenclaws-secrets` in the `ravenclaws`
namespace with the following keys:

| Key | Description | Required |
|---|---|---|
| `LITELLM_API_KEY` | API key for LiteLLM or OpenAI-compatible provider | Yes |
| `OPENAI_API_KEY` | API key for OpenAI (if using OpenAI directly) | No |
| `ANTHROPIC_API_KEY` | API key for Anthropic (if using Claude directly) | No |
| `OPENROUTER_API_KEY` | API key for OpenRouter (if using OpenRouter) | No |

**Example Secret YAML:**

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ravenclaws-secrets
  namespace: ravenclaws
type: Opaque
stringData:
  LITELLM_API_KEY: "sk-your-actual-key-here"
  # Add other keys as needed:
  # OPENAI_API_KEY: "sk-..."
  # ANTHROPIC_API_KEY: "sk-ant-..."
```

> **Security note:** Use `stringData` for convenience during development. For
> production, use `data` with base64-encoded values, or an external secrets
> manager like External Secrets Operator or Sealed Secrets.

### Environment Variables from Secrets

The deployment automatically maps Secret keys to environment variables via
`secretKeyRef`:

```yaml
env:
  - name: LITELLM_API_KEY
    valueFrom:
      secretKeyRef:
        name: ravenclaws-secrets
        key: LITELLM_API_KEY
```

To add additional environment variables from secrets, add more entries to the
`env` array in the Deployment spec.

### NetworkPolicy

The deployment includes a `NetworkPolicy` that:
- **Denies all ingress** by default (no inbound traffic allowed)
- **Allows DNS resolution** (UDP/TCP port 53)
- **Allows HTTPS egress** (TCP port 443) for LLM API calls
- **Allows HTTP egress** (TCP port 80) for local services like LiteLLM proxy

If your LLM API is on a specific CIDR range, uncomment and adjust the
`ipBlock` section in the NetworkPolicy.

### Container Image Size

The production image uses:
- **Distroless base** (`gcr.io/distroless/cc-debian12:nonroot`) — minimal OS surface
- **UPX compression** — binary is compressed to reduce image size
- **Conditional RavenFabric agent** — set `--build-arg INCLUDE_RAVENFABRIC=false`
  to exclude the optional RavenFabric agent binary (~15 MB savings)

Build a minimal image (no RavenFabric):
```bash
docker buildx build --build-arg INCLUDE_RAVENFABRIC=false -t ravenclaws:minimal .
```

### Testing Distroless Containers

The production image uses a **distroless base** (`gcr.io/distroless/cc-debian12:nonroot`)
which contains no shell, no `curl`, no `wget`, and no package manager. This is intentional
for security — minimal attack surface.

To test HTTP endpoints in a distroless container deployed on Kubernetes:

```bash
# Port-forward the service to your local machine
kubectl port-forward svc/ravenclaws 8080:8080

# Now test from your local machine (which has curl)
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics

# Test the chat endpoint
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Say hello"}'
```

For local Docker testing without Kubernetes:

```bash
# Run the container
docker run --rm -p 8080:8080 ghcr.io/egkristi/ravenclaws:latest --serve

# Test from your host (which has curl)
curl http://localhost:8080/health
```

## Next Steps

- [Configuration Reference](./configuration.md) — all config options
- [Swarm Mode Guide](./swarm-mode.md) — multi-agent orchestration
- [MCP Integration](./mcp-integration.md) — connect to MCP servers
- [Heartbeat Mode](./heartbeat-mode.md) — autonomous long-running agents
- [vLLM Integration](./vllm.md) — high-throughput local inference
- [llama.cpp Integration](./llamacpp.md) — lightweight CPU inference
- [Migration Guide](migration.md) — upgrading between versions
