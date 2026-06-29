# llama.cpp Integration

[llama.cpp](https://github.com/ggerganov/llama.cpp) is a lightweight, CPU-first inference engine for LLMs using GGUF format models. RavenClaws supports llama.cpp via the generic `openai-compatible` provider — no special configuration needed.

## Quick Start

### 1. Start llama.cpp server

```bash
# Download a GGUF model (e.g., Mistral 7B Instruct)
wget https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/mistral-7b-instruct-v0.3.Q4_K_M.gguf

# Start the llama.cpp server
llama-server -m mistral-7b-instruct-v0.3.Q4_K_M.gguf --port 8080
```

Or using Docker:

```bash
docker run --rm -p 8080:8080 \
  -v $(pwd)/models:/models \
  ghcr.io/ggerganov/llama.cpp:server \
  -m /models/mistral-7b-instruct-v0.3.Q4_K_M.gguf \
  --port 8080
```

### 2. Configure RavenClaws

```bash
# Via environment variables:
export RAVENCLAWS__LLM__PROVIDER="openai-compatible"
export RAVENCLAWS__LLM__ENDPOINT="http://localhost:8080/v1/chat/completions"
export RAVENCLAWS__LLM__MODEL="mistral-7b-instruct-v0.3"

# Run a task:
ravenclaws --exec "What is the capital of France?"
```

Or via config file (`ravenclaws.toml`):

```toml
[llm]
provider = "openai-compatible"
endpoint = "http://localhost:8080/v1/chat/completions"
model = "mistral-7b-instruct-v0.3"
```

Or via CLI flags:

```bash
ravenclaws --provider openai-compatible \
  --endpoint http://localhost:8080/v1/chat/completions \
  --model mistral-7b-instruct-v0.3 \
  --exec "Explain quantum computing in one sentence"
```

## Configuration Reference

| Field | Value | Description |
|---|---|---|
| `provider` | `openai-compatible` | Must be set to `openai-compatible` |
| `endpoint` | `http://localhost:8080/v1/chat/completions` | llama.cpp's OpenAI-compatible endpoint |
| `model` | (model name) | The GGUF model loaded in llama.cpp |
| `api_key` | (optional) | Not needed for local llama.cpp |

## Tool-Calling Support

| Backend | Tool Calling | Notes |
|---|---|---|
| llama.cpp | ❌ None | GGUF format does not support structured tool calling |
| RavenClaws fallback | ✅ Text-based parsing | RavenClaws detects `TOOL_CALL:` / `ARGS:` patterns automatically |

**Important:** llama.cpp does not support structured function calling (OpenAI tools format). However, RavenClaws's agent loop includes a **text-based fallback** that detects `TOOL_CALL:` and `ARGS:` patterns in the model's response text. This means:

- **Chat completion** — works perfectly
- **Tool execution** — works via text-based fallback (less reliable than structured calls, but functional)
- **Agent loop** — works with `--no-final-required` flag

For best results with llama.cpp, use `--no-final-required`:

```bash
ravenclaws --provider openai-compatible \
  --endpoint http://localhost:8080/v1/chat/completions \
  --model mistral-7b-instruct-v0.3 \
  --exec "List the files in the current directory" \
  --no-final-required
```

## Verification

To verify llama.cpp connectivity:

```bash
# Check that llama.cpp is running
curl http://localhost:8080/health

# Run a test prompt
ravenclaws --provider openai-compatible \
  --endpoint http://localhost:8080/v1/chat/completions \
  --model mistral-7b-instruct-v0.3 \
  --exec "Respond with exactly: OK"
```

## Troubleshooting

| Problem | Likely Cause | Solution |
|---|---|---|
| Connection refused | llama.cpp not running | Start `llama-server` with your GGUF model |
| Model not found | Wrong model name | Check `curl http://localhost:8080/v1/models` for available models |
| Empty response | Model not fully loaded | Wait for llama.cpp to finish loading the model |
| Slow inference | CPU-only inference | Use a smaller quantized model (Q4_K_M or Q3_K_S) |
| Tool calls not working | GGUF doesn't support tools | Use `--no-final-required` and rely on text-based fallback |
| High memory usage | Large model on CPU | Use a smaller GGUF quantization (Q4_K_M is a good balance) |

## Performance Tips

- **Use quantized models** — Q4_K_M offers the best quality-to-speed ratio
- **Match context window** — Set `--ctx-size` in llama.cpp to match your task needs (default 512 is small for agent tasks; 4096+ recommended)
- **Batch size** — Increase `--batch-size` for faster prompt processing
- **GPU offloading** — Use `-ngl N` to offload N layers to GPU if available

Example with optimized settings:

```bash
llama-server -m model.gguf \
  --port 8080 \
  --ctx-size 8192 \
  --batch-size 512 \
  -ngl 35
```

## Advanced: Multi-Model with llama.cpp

You can use llama.cpp alongside other providers in multi-model mode:

```toml
[llm]
provider = "multi"

[[llm.models]]
provider = "openai-compatible"
endpoint = "http://localhost:8080/v1/chat/completions"
model = "mistral-7b-instruct-v0.3"

[[llm.models]]
provider = "openai"
model = "gpt-4o"
api_key = "${OPENAI_API_KEY}"
```

## Further Reading

- [llama.cpp Documentation](https://github.com/ggerganov/llama.cpp)
- [OpenAI-Compatible Provider](./configuration.md#openai-compatible-provider)
- [Configuration Reference](./configuration.md)
