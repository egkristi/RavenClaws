# Server Mode

RavenClaws can run as a **long-lived HTTP server** that exposes agent capabilities via a REST API. This enables integration with external systems, web UIs, CI/CD pipelines, and microservice architectures.

## Quick Start

```bash
# Start the server on the default port (8080)
ravenclaws --serve

# With a custom port
RAVENCLAWS__RUNTIME__PORT=9090 ravenclaws --serve

# With a config file
ravenclaws --serve --config /path/to/config.toml
```

## Endpoints

### `GET /health` — Liveness check

Returns `200 OK` with a JSON body indicating the server is alive.

```json
{
  "status": "ok"
}
```

### `GET /ready` — Readiness check

Returns `200 OK` once the server is fully initialized (LLM client loaded, tools registered). Returns `503 Service Unavailable` during startup.

```json
{
  "status": "ready"
}
```

### `GET /health/deep` — Deep health check

Returns detailed health information including uptime, request count, and LLM provider status.

```json
{
  "status": "ok",
  "uptime_secs": 3600,
  "requests_served": 42,
  "llm_provider": "openai",
  "llm_model": "gpt-4o",
  "tools_registered": 5
}
```

### `GET /metrics` — Prometheus-style metrics

Returns basic operational metrics:

```
# HELP ravenclaws_requests_total Total HTTP requests served
# TYPE ravenclaws_requests_total counter
ravenclaws_requests_total 42
# HELP ravenclaws_uptime_seconds Server uptime in seconds
# TYPE ravenclaws_uptime_seconds gauge
ravenclaws_uptime_seconds 3600
```

### `POST /chat` — Chat completion

Send a prompt and receive a streaming or non-streaming response from the agent.

**Request:**
```json
{
  "prompt": "What is the capital of France?",
  "stream": false
}
```

**Response:**
```json
{
  "response": "The capital of France is Paris.",
  "model": "gpt-4o",
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 8
  }
}
```

### `POST /execute` — Execute a task with tools

Run a task that may involve tool calls (web search, file read/write, shell commands). Returns a task ID for polling.

**Request:**
```json
{
  "prompt": "Search the web for latest Rust news",
  "tools": ["web_search"]
}
```

**Response:**
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running"
}
```

### `GET /tasks/{id}` — Poll task status

Check the status of an async task started via `/execute`.

**Response (running):**
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running"
}
```

**Response (completed):**
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": "Latest Rust news: Rust 1.86 released with ...",
  "tool_calls": [
    {
      "tool": "web_search",
      "arguments": {"query": "latest Rust news"},
      "result": "..."
    }
  ]
}
```

### `GET /tools` — List available tools

Returns all registered tools with their names and descriptions.

```json
{
  "tools": [
    {"name": "web_search", "description": "Search the web for information"},
    {"name": "read_file", "description": "Read a file from the filesystem"},
    {"name": "write_file", "description": "Write content to a file"},
    {"name": "shell", "description": "Execute a shell command"},
    {"name": "web_fetch", "description": "Fetch a URL and return its content"}
  ]
}
```

### `POST /tools/{name}` — Execute a specific tool

Execute a single tool by name with provided arguments.

**Request:**
```json
{
  "arguments": {
    "query": "Rust programming language"
  }
}
```

**Response:**
```json
{
  "tool": "web_search",
  "result": "Rust is a multi-paradigm, general-purpose programming language...",
  "duration_ms": 450
}
```

## Configuration

### Port

The server port defaults to `8080` and can be configured via:

- **Config file:** `[runtime] port = 9090`
- **Environment variable:** `RAVENCLAWS__RUNTIME__PORT=9090`
- **CLI flag:** Not directly available — use env var or config file

### TLS

TLS is not built into the server. For production deployments, place RavenClaws behind a reverse proxy (nginx, Caddy, Cloudflare Tunnel) that terminates TLS.

### CORS

The server does not include built-in CORS headers. When calling from a browser, use a reverse proxy to add CORS headers as needed.

## Deployment

### Docker

```bash
docker run -d \
  --name ravenclaws \
  -p 8080:8080 \
  -e RAVENCLAWS__RUNTIME__PORT=8080 \
  -e OPENAI_API_KEY=sk-... \
  ghcr.io/egkristi/ravenclaws:latest \
  --serve
```

### Kubernetes

The included Helm chart supports server mode. Set `mode: serve` in your values:

```yaml
mode: serve
config:
  runtime:
    port: 8080
```

See the [Helm chart](https://github.com/egkristi/RavenClaws/tree/master/charts/ravenclaws) for full configuration options.

### Systemd

```ini
[Unit]
Description=RavenClaws Agent Server
After=network.target

[Service]
ExecStart=/usr/local/bin/ravenclaws --serve
Environment=RAVENCLAWS__RUNTIME__PORT=8080
Environment=OPENAI_API_KEY=sk-...
Restart=always
User=ravenclaws

[Install]
WantedBy=multi-user.target
```

## SIGHUP Reload

The server supports hot-reloading configuration on `SIGHUP`:

```bash
kill -HUP <pid>
```

On receiving `SIGHUP`, the server re-reads the configuration file and logs the result. Full hot-reload of LLM clients and tool registries is planned for a future release.

## Graceful Shutdown

The server handles `SIGTERM` and `SIGINT` (Ctrl+C) for graceful shutdown. In-flight requests are given up to 5 seconds to complete before the process exits.

## Examples

### cURL

```bash
# Health check
curl http://localhost:8080/health

# Chat
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello, who are you?"}'

# Execute a task
curl -X POST http://localhost:8080/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt": "What is 2+2?"}'

# List tools
curl http://localhost:8080/tools
```

### Python

```python
import requests

BASE = "http://localhost:8080"

# Health check
print(requests.get(f"{BASE}/health").json())

# Chat
resp = requests.post(f"{BASE}/chat", json={"prompt": "Hello!"})
print(resp.json()["response"])
```

### Node.js

```javascript
const BASE = "http://localhost:8080";

// Health check
const health = await fetch(`${BASE}/health`).then(r => r.json());
console.log(health);

// Chat
const chat = await fetch(`${BASE}/chat`, {
  method: "POST",
  headers: {"Content-Type": "application/json"},
  body: JSON.stringify({prompt: "Hello!"})
}).then(r => r.json());
console.log(chat.response);
```
