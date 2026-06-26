use ravenclaws::heartbeat::{HeartbeatAgent, HeartbeatConfig};
/// Heartbeat agent example demonstrating autonomous long-running operation.
///
/// This shows how to:
/// - Configure and start a heartbeat agent
/// - Implement the assess→plan→act→persist→sleep cycle
/// - Handle state persistence for crash recovery
///
/// Run with: cargo run --example heartbeat
///
/// Requires a config file `ravenclaws.toml` or environment variables.
use ravenclaws::{create_client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load(None)?;

    // Create an LLM client
    let client = create_client(&config.llm)?;

    // Configure the heartbeat agent
    let heartbeat_config = HeartbeatConfig {
        goal: "Monitor the system and report status".to_string(),
        tick_interval_secs: 60, // Check every 60 seconds
        max_iterations_per_tick: 5,
        workdir: "/tmp/ravenclaws-heartbeat".to_string(),
        max_ticks: 5, // Run 5 ticks then stop
        enable_tools: true,
    };

    // Create and start the heartbeat agent
    let mut agent = HeartbeatAgent::new(
        client,
        heartbeat_config,
        None, // auto-generate session ID
    )
    .await?;

    println!("Starting heartbeat agent (5 ticks, 60s interval)...");
    println!("Session ID: {}", agent.id());
    println!("Press Ctrl+C to stop early.\n");

    // Run the heartbeat loop
    let result = agent.run().await?;

    println!("\nHeartbeat agent completed: {}", result);

    Ok(())
}
