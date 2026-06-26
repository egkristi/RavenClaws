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
