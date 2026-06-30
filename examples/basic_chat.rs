/// Basic chat example using RavenClaws as a library.
///
/// This demonstrates:
/// - Loading configuration
/// - Creating an LLM client
/// - Sending a chat message and getting a response
///
/// Run with: cargo run --example basic_chat
///
/// Requires a config file `ravenclaws.toml` or environment variables:
///   RAVENCLAWS__LLM__PROVIDER=openai
///   RAVENCLAWS__LLM__API_KEY=sk-...
///   RAVENCLAWS__LLM__MODEL=gpt-4o-mini
use ravenclaws::{create_client, ChatMessage, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from file or environment
    let config = Config::load(None)?;

    // Create an LLM client
    let client = create_client(&config.llm)?;

    // Send a chat message
    let response = client
        .chat(vec![ChatMessage::new(
            "user",
            "What is the capital of France?",
        )])
        .await?;

    // Print the response
    println!("Response: {}", response.choices[0].message.content);

    Ok(())
}
