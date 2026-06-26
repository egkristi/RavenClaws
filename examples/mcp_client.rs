/// MCP client example demonstrating how to connect to an MCP server.
///
/// This shows how to:
/// - Connect to an MCP server via stdio
/// - Discover available tools
/// - Call MCP tools from Rust code
///
/// Run with: cargo run --example mcp_client
///
/// Requires an MCP server command (e.g., the filesystem server):
///   cargo run --example mcp_client -- "npx @modelcontextprotocol/server-filesystem /tmp"
use ravenclaws::mcp::{McpClient, McpTransportConfig};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the MCP server command from CLI args
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: mcp_client <mcp-server-command> [args...]");
        eprintln!("Example: mcp_client 'npx @modelcontextprotocol/server-filesystem /tmp'");
        std::process::exit(1);
    }

    let server_command = &args[1];
    let server_args: Vec<String> = args[2..].to_vec();

    // Configure MCP transport
    let transport_config = McpTransportConfig::Stdio {
        command: server_command.clone(),
        args: server_args,
        env: HashMap::new(),
    };

    // Create and connect to the MCP server
    let mut client = McpClient::new(transport_config);
    client.connect().await?;
    println!("Connected to MCP server: {}", server_command);

    // Discover available tools
    client.discover_tools().await?;
    let tools = client.get_tools().await;
    println!("\nAvailable tools ({}):", tools.len());
    for tool in &tools {
        println!("  - {}: {:?}", tool.name, tool.description);
    }

    // Call a tool (example: list directory)
    if tools.iter().any(|t| t.name == "list_directory") {
        let result = client
            .call_tool("list_directory", Some(serde_json::json!({"path": "/tmp"})))
            .await?;
        println!("\nTool result: {:?}", result);
    }

    println!("\nMCP client session complete.");

    Ok(())
}
