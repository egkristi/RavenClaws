/// Agent loop example demonstrating tool-use with the ReAct pattern.
///
/// This shows how to:
/// - Set up an agent loop with tools
/// - Configure a policy engine for security
/// - Run a multi-step task with tool calls
///
/// Run with: cargo run --example agent_loop
///
/// Requires a config file `ravenclaws.toml` or environment variables.
use ravenclaws::{create_client, run_agent_loop, AgentLoopConfig, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load(None)?;

    // Create an LLM client
    let client = create_client(&config.llm)?;

    // Configure the agent loop
    let loop_config = AgentLoopConfig {
        max_iterations: 10,
        enable_tools: true,
        require_approval: false,
        prompt_injection_protection: true,
        token_lifetime_secs: 0,
        no_final_required: false,
        fallback_chain: None,
        token_budget: None,
        ravenfabric: None,
        checkpoint_dir: None,
        session_id: None,
        metrics_callback: None,
        load_manager: None,
        retry_config: None,
        healing_engine: None,
    };

    // Run the agent loop
    let result = run_agent_loop(
        client,
        "Create a file called hello.txt with the text 'Hello, World!'",
        &config.llm.system_prompt,
        loop_config,
    )
    .await?;

    println!("Final response: {}", result);

    Ok(())
}
