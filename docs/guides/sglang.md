# SGLang Integration

[SGLang](https://github.com/sgl-project/sglang) is a fast inference engine for large language models and vision-language models. RavenClaws supports SGLang as a **first-class provider** (`provider = "sglang"`, default endpoint `http://localhost:30000`), as well as via the generic `openai-compatible` provider.

## Quick Start

### 1. Start SGLang

```bash
# Pull and run SGLang with a model (e.g., Qwen2.5 7B)
docker run --rm --gpus all \
  -p 30000:30000 \
  lmsysorg/sglang:latest \
  python3 -m sglang.launch_server \
  --model-path Qwen/Qwen2.5-7B-Instruct \
  --port 30000
```

Or run SGLang directly:

```bash
pip install "sglang[all]"
python -m sglang.launch_server \
  --model-path Qwen/Qwen2.5-7B-Instruct \
  --port 30000
```

### 2. Configure RavenClaws (first-class `sglang` provider)

```toml
[llm]
provider = "sglang"
endpoint = "http://localhost:30000"   # optional — this is the default
model = "Qwen/Qwen2.5-7B-Instruct"
```

Or via environment variables:

```bash
export RAVENCLAWS__LLM__PROVIDER="sglang"
export RAVENCLAWS__LLM__MODEL="Qwen/Qwen2.5-7B-Instruct"

# Run a task:
ravenclaws --exec "What is the capital of France?"
```

Alternatively, the generic `openai-compatible` provider also works:

```bash
export RAVENCLAWS__LLM__PROVIDER="openai-compatible"
export RAVENCLAWS__LLM__ENDPOINT="http://localhost:30000/v1/chat/completions"
export RAVENCLAWS__LLM__MODEL="Qwen/Qwen2.5-7B-Instruct"
```

## Configuration Reference

| Field | Default | Description |
|---|---|---|
| `provider` | — | `sglang` (first-class) or `openai-compatible` |
| `endpoint` | `http://localhost:30000` | SGLang's OpenAI-compatible endpoint |
| `model` | (model name) | The model loaded in SGLang |
| `api_key` | (optional) | Some SGLang deployments require an API key |

## Tool Calling

SGLang serves an OpenAI-compatible API, so tool calling works identically to the
`openai-compatible` provider:

- **Structured tool calls** — if SGLang returns `tool_calls` in the response, they
  are executed directly.
- **Text-based fallback** — if the model doesn't support structured tool calls,
  RavenClaws parses `TOOL_CALL:` / `ARGS:` patterns from the response text.

## Verifying Connectivity

```bash
curl http://localhost:30000/v1/models
```

See the [`scripts/lib/test-provider-vllm.sh`](../scripts/lib/test-provider-vllm.sh)
verification script for the same pattern applied to OpenAI-compatible endpoints.

## Multi-model Example

```toml
[[llms]]
provider = "sglang"
endpoint = "http://localhost:30000"
model = "Qwen/Qwen2.5-7B-Instruct"

[[llms]]
provider = "vllm"
endpoint = "http://localhost:8000"
model = "mistralai/Mistral-7B-Instruct-v0.3"

[[llms]]
provider = "ollama"
endpoint = "http://localhost:11434"
model = "llama3.1"
```
