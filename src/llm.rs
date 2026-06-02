//! Multi-provider LLM client integration
//!
//! Supports LiteLLM, OpenAI, OpenRouter, and Ollama with a unified trait-based API.

use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::sync::Arc;

use crate::config::{LLMConfig, LLMProvider};

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Authentication failed")]
    AuthFailed,
    
    #[error("Provider not supported: {0}")]
    ProviderNotSupported(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Trait for LLM providers — unified interface across all backends
#[async_trait::async_trait]
pub trait LLMProviderTrait: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError>;
    fn provider_name(&self) -> &str;
    fn model(&self) -> &str;
}

/// LiteLLM client (OpenAI-compatible API)
pub struct LiteLLMClient {
    client: Client,
    config: LLMConfig,
}

impl LiteLLMClient {
    pub fn new(config: &LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            config: config.clone(),
        }
    }
    
    fn build_request(&self, messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: None,
        }
    }
    
    async fn send_request(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let mut req = self.client
            .post(format!("{}/v1/chat/completions", self.config.endpoint.trim_end_matches('/')))
            .json(&request);
        
        if let Some(ref key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
        
        self.handle_response(response).await
    }
    
    async fn handle_response(&self, response: Response) -> Result<ChatResponse, LLMError> {
        let status = response.status();
        
        if status.is_success() {
            response
                .json::<ChatResponse>()
                .await
                .map_err(|e| LLMError::InvalidResponse(e.to_string()))
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(LLMError::AuthFailed)
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(LLMError::RateLimited)
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(LLMError::RequestFailed(format!("{}: {}", status, body)))
        }
    }
}

#[async_trait::async_trait]
impl LLMProviderTrait for LiteLLMClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError> {
        let request = self.build_request(messages);
        self.send_request(request).await
    }
    
    fn provider_name(&self) -> &str {
        "litellm"
    }
    
    fn model(&self) -> &str {
        &self.config.model
    }
}

/// OpenRouter client (OpenAI-compatible with model routing)
pub struct OpenRouterClient {
    client: Client,
    config: LLMConfig,
}

impl OpenRouterClient {
    pub fn new(config: &LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProviderTrait for OpenRouterClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: None,
        };
        
        let mut req = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_ref().unwrap_or(&"".to_string())))
            .header("HTTP-Referer", "https://github.com/egkristi/RavenClaw")
            .header("X-Title", "RavenClaw")
            .json(&request);
        
        let response = req
            .send()
            .await
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
        
        let status = response.status();
        
        if status.is_success() {
            response
                .json::<ChatResponse>()
                .await
                .map_err(|e| LLMError::InvalidResponse(e.to_string()))
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(LLMError::AuthFailed)
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(LLMError::RateLimited)
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(LLMError::RequestFailed(format!("{}: {}", status, body)))
        }
    }
    
    fn provider_name(&self) -> &str {
        "openrouter"
    }
    
    fn model(&self) -> &str {
        &self.config.model
    }
}

/// Ollama client (local/self-hosted models)
pub struct OllamaClient {
    client: Client,
    config: LLMConfig,
}

impl OllamaClient {
    pub fn new(config: &LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProviderTrait for OllamaClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError> {
        // Ollama uses slightly different format
        #[derive(Serialize)]
        struct OllamaRequest {
            model: String,
            messages: Vec<ChatMessage>,
            stream: bool,
        }
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
        };
        
        let response = self.client
            .post(format!("{}/api/chat", self.config.endpoint.trim_end_matches('/')))
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
        
        let status = response.status();
        
        if status.is_success() {
            // Ollama returns a different structure, convert to our standard format
            #[derive(Deserialize)]
            struct OllamaResponse {
                model: String,
                message: ChatMessage,
                done: bool,
            }
            
            let ollama_resp = response
                .json::<OllamaResponse>()
                .await
                .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;
            
            Ok(ChatResponse {
                id: format!("ollama-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".to_string(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                model: ollama_resp.model,
                choices: vec![Choice {
                    index: 0,
                    message: ollama_resp.message,
                    finish_reason: if ollama_resp.done { Some("stop".to_string()) } else { None },
                }],
                usage: None, // Ollama doesn't always provide usage
            })
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(LLMError::AuthFailed)
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(LLMError::RequestFailed(format!("{}: {}", status, body)))
        }
    }
    
    fn provider_name(&self) -> &str {
        "ollama"
    }
    
    fn model(&self) -> &str {
        &self.config.model
    }
}

/// OpenAI native client
pub struct OpenAIClient {
    client: Client,
    config: LLMConfig,
}

impl OpenAIClient {
    pub fn new(config: &LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProviderTrait for OpenAIClient {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: None,
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_ref().unwrap_or(&"".to_string())))
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
        
        let status = response.status();
        
        if status.is_success() {
            response
                .json::<ChatResponse>()
                .await
                .map_err(|e| LLMError::InvalidResponse(e.to_string()))
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(LLMError::AuthFailed)
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(LLMError::RateLimited)
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(LLMError::RequestFailed(format!("{}: {}", status, body)))
        }
    }
    
    fn provider_name(&self) -> &str {
        "openai"
    }
    
    fn model(&self) -> &str {
        &self.config.model
    }
}

/// Factory function to create the appropriate client based on provider type
pub fn create_client(config: &LLMConfig) -> Result<Arc<dyn LLMProviderTrait>, LLMError> {
    match config.provider {
        LLMProvider::LiteLLM => Ok(Arc::new(LiteLLMClient::new(config))),
        LLMProvider::OpenRouter => Ok(Arc::new(OpenRouterClient::new(config))),
        LLMProvider::Ollama => Ok(Arc::new(OllamaClient::new(config))),
        LLMProvider::OpenAI => Ok(Arc::new(OpenAIClient::new(config))),
    }
}

/// Multi-model manager for handling multiple providers simultaneously
pub struct MultiModelManager {
    clients: Vec<Arc<dyn LLMProviderTrait>>,
}

impl MultiModelManager {
    pub fn new(configs: Vec<LLMConfig>) -> Result<Self, LLMError> {
        let clients: Result<Vec<_>, _> = configs.iter().map(create_client).collect();
        Ok(Self {
            clients: clients?,
        })
    }
    
    pub fn get_client(&self, index: usize) -> Option<&Arc<dyn LLMProviderTrait>> {
        self.clients.get(index)
    }
    
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
    
    /// Round-robin selection for load balancing
    pub fn next_client(&self, last_index: usize) -> &Arc<dyn LLMProviderTrait> {
        let next = (last_index + 1) % self.clients.len();
        &self.clients[next]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_litellm_client_creation() {
        let config = LLMConfig {
            provider: LLMProvider::LiteLLM,
            endpoint: "http://localhost:4000".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("test".to_string()),
            timeout_secs: 30,
        };
        
        let _client = LiteLLMClient::new(&config);
    }
    
    #[test]
    fn test_ollama_client_creation() {
        let config = LLMConfig {
            provider: LLMProvider::Ollama,
            endpoint: "http://localhost:11434".to_string(),
            model: "llama3.1".to_string(),
            api_key: None,
            timeout_secs: 30,
        };
        
        let _client = OllamaClient::new(&config);
    }
}
