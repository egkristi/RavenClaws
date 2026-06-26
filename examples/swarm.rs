use ravenclaws::swarm::{SwarmConfig, SwarmOrchestrator, SwarmTopology, WorkerProfile};
/// Swarm mode example using RavenClaws as a library.
///
/// This demonstrates how to:
/// - Configure a swarm of agents with different personas
/// - Run parallel agents on the same task
/// - Collect results from all agents
///
/// Run with: cargo run --example swarm
///
/// Requires a config file `ravenclaws.toml` or environment variables.
use ravenclaws::{create_client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load(None)?;

    // Create an LLM client
    let client = create_client(&config.llm)?;

    // Define worker profiles using built-in factory methods
    let profiles = vec![
        WorkerProfile::researcher(),
        WorkerProfile::executor(),
        WorkerProfile::reviewer(),
    ];

    // Configure the swarm
    let swarm_config = SwarmConfig {
        topology: SwarmTopology::Star,
        max_depth: 3,
        max_workers: 100,
        profiles,
        dynamic_role_assignment: true,
        enable_agent_communication: false,
        enable_health_monitoring: false,
    };

    // Create and run the orchestrator
    let mut orchestrator = SwarmOrchestrator::new(
        swarm_config,
        Some(client),
        None, // no multi-model manager
        None, // no RavenFabric client
    );
    let result = orchestrator
        .orchestrate("Review the Rust programming language for systems programming")
        .await?;

    // Print the result
    println!("Swarm result:\n{}", result);

    Ok(())
}
