# Changelog

All notable changes to RavenClaw will be documented in this file.

## [Unreleased]

### Added
- `--exec` mode now fully wired — one-shot command execution with response printed to stdout
- Comprehensive Rust unit tests: 39 tests across all modules (was 3)
- Manual `Default` implementations for `RavenFabricConfig`, `SecurityConfig`, and `RuntimeConfig` matching serde defaults
- CLI `--version` now uses `env!("CARGO_PKG_VERSION")` instead of hardcoded string
- Test coverage for config validation, LLM client creation, error types, CLI argument parsing, and agent stubs

### Fixed
- `--exec` dead code — CLI arg was parsed but never used; now sends prompt to LLM and prints response
- Swarm/supervisor stubs now return `Err(RavenClawError::CommandExecution(...))` instead of silently exiting 0
- All 4 LLM client constructors (`LiteLLMClient`, `OpenRouterClient`, `OllamaClient`, `OpenAIClient`) now return `Result<Self, LLMError>` instead of calling `.expect()`
- `create_client()` factory function propagates client construction errors via `?`
- Verification `check_llm_response_quality` now handles `--exec` mode output (stdout-based responses)
- `Cargo.lock` removed from `.gitignore` and committed for reproducible builds

### Changed
- `RavenFabricConfig`, `SecurityConfig`, `RuntimeConfig` now use manual `Default` impls instead of `#[derive(Default)]` to ensure serde defaults match Rust defaults
