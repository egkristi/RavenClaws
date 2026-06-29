# vLLM Integration

[vLLM](https://github.com/vllm-project/vllm) is a high-throughput, memory-efficient inference engine for LLMs. RavenClaws supports vLLM via the generic `openai-compatible` provider — no special configuration needed.

## Quick Start

### 1. Start vLLM

```bash
# Pull and run vLLM with a model (e.g., Mistral 7B)
docker run --rm --gpus all \
  -p 8000:8000 \
  vllm/vllm-openai:latest \
  --model mistralai/Mistral-7B-Instruct-v0.3
```

Or run vLLM directly:

```bash
pip install vllm
python -m vllm.entrypoints.openai.api_server \
  --model mistralai/Mistral-7B-Instruct-v0.3 \
  --port 8000
```

### 2. Configure RavenClaws

```bash
# Via environment variables:
export RAVENCLAWS__LLM__PROVIDER="openai-compatible"
export RAVENCLAWS__LLM__ENDPOINT="http://localhost:8000/v1/chat/completions"
export RAVENCLAWS__LLM__MODEL="mistralai/Mistral-7B-Instruct-v0.3"

# Run a task:
ravenclaws --exec "What is the capital of France?"
```

Or via config file (`ravenclaws.toml`):

```toml
[llm]
provider = "openai-compatible"
endpoint = "http://localhost:8000/v1/chat/completions"
model = "mistralai/Mistral-7B-Instruct-v0.3"
```

Or via CLI flags:

```bash
ravenclaws --provider openai-compatible \
  --endpoint http://localhost:8000/v1/chat/completions \
  --model mistralai/Mistral-7B-Instruct-v0.3 \
  --exec "Explain quantum computing in one sentence"
```

## Configuration Reference

| Field | Value | Description |
|---|---|---|
| `provider` | `openai-compatible` | Must be set to `openai-compatible` |
| `endpoint` | `http://localhost:8000/v1/chat/completions` | vLLM's OpenAI-compatible endpoint |
| `model` | (model name) | The model loaded in vLLM |
| `api_key` | (optional) | Some vLLM deployments require an API key |

## Tool-Calling Support

| Backend | Tool Calling | Notes |
|---|---|---|
| vLLM (with tool-supporting model) | ⚠️ Partial | Supports OpenAI tools format; quality varies by model |
| vLLM (without tool support) | ❌ Falls back to text parsing | RavenClaws detects `TOOL_CALL:` patterns automatically |

RavenClaws's agent loop handles both modes:
- **Structured tool calls** — if vLLM returns `tool_calls` in the response, they are executed directly
- **Text-based fallback** — if vLLM doesn't support structured tool calls, RavenClaws parses `TOOL_CALL:` / `ARGS:` patterns from the response text

## Verification

To verify vLLM connectivity:

```bash
# Check that vLLM is running
curl http://localhost:8000/health

# Run a test prompt
ravenclaws --provider openai-compatible \
  --endpoint http://localhost:8000/v1/chat/completions \
  --model mistralai/Mistral-7B-Instruct-v0.3 \
  --exec "Respond with exactly: OK"
```

## Troubleshooting

| Problem | Likely Cause | Solution |
|---|---|---|
| Connection refused | vLLM not running | Start vLLM with `docker run ... vllm/vllm-openai:latest` |
| Model not found | Wrong model name | Check `curl http://localhost:8000/v1/models` for available models |
| Empty response | Model not fully loaded | Wait for vLLM to finish loading (check logs for "Uvicorn running on") |
| Slow first response | Model loading into GPU | First request is slow as model loads; subsequent requests are fast |
| Tool calls not working | Model doesn't support tools | Use a model that supports OpenAI tool calling, or rely on text-based fallback |

## Advanced: Multi-Model with vLLM

You can use vLLM alongside other providers in multi-model mode:

```toml
[llm]
provider = "multi"

[[llm.models]]
provider = "openai-compatible"
endpoint = "http://localhost:8000/v1/chat/completions"
model = "mistralai/Mistral-7B-Instruct-v0.3"

[[llm.models]]
provider = "openai"
model = "gpt-4o"
api_key = "${OPENAI_API_KEY}"
```

## Further Reading

- [vLLM Documentation](https://docs.vllm.ai/)
- [OpenAI-Compatible Provider](./configuration.md#openai-compatible-provider)
- [Configuration Reference](./configuration.md)
