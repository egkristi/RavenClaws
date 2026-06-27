//! # RavenClaws
//!
//! Lightweight, secure Rust agent framework with multi-provider LLM support.
//!
//! RavenClaws is a single-binary agent runtime that supports:
//! - **Single agent mode** — one prompt, one response
//! - **Swarm mode** — multiple parallel agents with different personas
//! - **Supervisor mode** — task decomposition with sub-agent spawning
//! - **Heartbeat mode** — autonomous long-running agents
//! - **REPL mode** — interactive conversation
//! - **Server mode** — HTTP server with health/metrics endpoints
//! - **MCP server mode** — expose tools over stdio via MCP protocol
//!
//! ## Architecture
//!
//! The crate is organized into 18 modules:
//!
//! | Module | Purpose |
//! |---|---|
//! | [`agent`] | Agent implementations, agent loop, conversation memory |
//! | [`llm`] | LLM provider abstraction + 5 client implementations |
//! | [`config`] | Configuration structs, TOML/env loading, validation |
//! | [`tools`] | Tool abstraction, registry, 5 built-in tools |
//! | [`policy`] | Deny-by-default policy engine |
//! | [`sandbox`] | Sandboxed execution (workdir jail, resource limits) |
//! | [`audit`] | Tamper-evident audit log (HMAC-SHA256 chained) |
//! | [`mcp`] | MCP client + server (JSON-RPC 2.0 over stdio) |
//! | [`swarm`] | Swarm orchestration, worker profiles, health monitoring |
//! | [`heartbeat`] | Autonomous heartbeat agent |
//! | [`background`] | Background task manager with disk persistence |
//! | [`scheduler`] | Scheduling & triggers (cron, webhook, file-watch) |
//! | [`server`] | HTTP server mode (health, readiness, metrics) |
//! | [`telemetry`] | OpenTelemetry tracing (OTLP gRPC/stdout) |
//! | [`ravenfabric`] | RavenFabric mesh client |
//! | [`eval`] | Eval harness with assertions and run traces |
//! | [`error`] | Unified error types |
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ravenclaws::config::Config;
//! use ravenclaws::llm::{create_client, LLMProviderTrait};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::load(None)?;
//! let llm = create_client(&config.llm)?;
//! let response = llm.chat(vec![
//!     ravenclaws::llm::ChatMessage {
//!         role: "user".to_string(),
//!         content: "Hello!".to_string(),
//!     },
//! ]).await?;
//! println!("{}", response.choices[0].message.content);
//! # Ok(())
//! # }
//! ```
//!
//! ## Security
//!
//! RavenClaws uses a deny-by-default security model:
//! - All tool calls are validated by [`PolicyEngine`] before execution
//! - Shell commands execute in a [`Sandbox`] with resource limits
//! - All operations are logged to a tamper-evident [`AuditLog`]
//! - API keys are zeroized on drop
//!
//! ## Feature Flags
//!
//! - `otel-grpc` (default) — OpenTelemetry tracing via OTLP gRPC exporter
//! - `otel-stdout` — OpenTelemetry tracing via stdout exporter
//!
//! ## Minimum Supported Rust Version (MSRV)
//!
//! Rust 1.86 or later. This crate uses edition 2021.
//!
//! ## Semver Guarantees
//!
//! RavenClaws follows semantic versioning. The public API consists of all items
//! documented in this module and re-exported below. Items marked `#[doc(hidden)]`
//! or in `__private` modules are not part of the public API and may change in
//! minor releases.
//!
//! All public enums and structs are `#[non_exhaustive]` — new variants/fields may
//! be added in minor releases. Match statements on enums must include a wildcard
//! arm, and struct literals must use `..` syntax.

pub mod agent;
pub mod audit;
pub mod background;
pub mod config;
pub mod error;
pub mod eval;
pub mod heartbeat;
pub mod llm;
pub mod mcp;
pub mod policy;
pub mod ravenfabric;
pub mod sandbox;
pub mod scheduler;
pub mod server;
pub mod swarm;
pub mod telemetry;
pub mod tools;

// ── Re-exports of commonly used types ──────────────────────────────────────

pub use agent::{
    run_agent_loop, run_agent_loop_with_mcp, run_agent_loop_with_mcp_and_registry,
    run_agent_loop_with_registry, AgentLoopConfig, ConversationMemory,
};
pub use audit::AuditLog;
pub use background::BackgroundTaskManager;
pub use config::{Config, LLMConfig, LLMProvider, RuntimeConfig, SecurityConfig};
pub use error::RavenClawsError;
pub use eval::EvalRunner;
pub use heartbeat::HeartbeatAgent;
pub use llm::{create_client, ChatMessage, ChatResponse, LLMProviderTrait, MultiModelManager};
pub use mcp::{McpClient, McpServer};
pub use policy::PolicyEngine;
pub use ravenfabric::RavenFabricClient;
pub use sandbox::Sandbox;
pub use scheduler::Scheduler;
pub use server::run_server;
pub use swarm::SwarmOrchestrator;
pub use telemetry::TelemetryGuard;
pub use tools::{ToolCall, ToolImpl, ToolRegistry, ToolResult};
