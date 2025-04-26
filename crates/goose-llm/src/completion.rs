use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::{Provider, ProviderUsage};
use goose::providers::errors::ProviderError;
use goose::providers;
use mcp_core::tool::Tool;

use crate::prompt_template;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    message: Message,
    usage: ProviderUsage
}

impl CompletionResponse {
    pub fn new(message: Message, usage: ProviderUsage) -> Self {
        Self {
            message,
            usage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    name: String,
    instructions: Option<String>, 
    tools: Vec<Tool>,
}

impl Extension {
    pub fn new(name: String, instructions: Option<String>, tools: Vec<Tool>) -> Self {
        Self {
            name,
            instructions,
            tools,
        }
    }

    pub fn get_prefixed_tools(&self) -> Vec<Tool> {
        self.tools.iter().map(|tool| {
            let mut prefixed_tool = tool.clone();
            prefixed_tool.name = format!("{}__{}", self.name, tool.name);
            prefixed_tool
        }).collect()
    }
}


/// Public API for the Goose LLM completion function
pub async fn completion(
    provider_name: &str,
    model_config: ModelConfig,
    system_preamble: &str,
    messages: &[Message],
    extensions: &[Extension]
) -> Result<CompletionResponse, ProviderError> {
    // Create the appropriate provider based on the provider_name parameter
    let provider = create_provider(provider_name, model_config)?;
    let system_prompt = construct_system_prompt(system_preamble, extensions);
    // println!("\nSystem prompt: {}\n", system_prompt);

    let tools = extensions.iter()
        .flat_map(|ext| ext.get_prefixed_tools())
        .collect::<Vec<_>>();
    let (response, usage) = provider.complete(&system_prompt, messages, &tools).await?;
    let result = CompletionResponse::new(response.clone(), usage.clone());

    Ok(result)
}


fn construct_system_prompt(system_preamble: &str, extensions: &[Extension]) -> String {
    let mut context: HashMap<&str, Value> = HashMap::new();

    context.insert("system_preamble", Value::String(system_preamble.to_string()));
    context.insert("extensions", serde_json::to_value(extensions).unwrap());

    let current_date_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    context.insert("current_date_time", Value::String(current_date_time));
    
    prompt_template::render_global_file("system.md", &context)
                .expect("Prompt should render")
}

/// Creates a provider based on the provider name and model config
/// This function does NOT use Config::global() and instead relies on the
/// environment variables being set directly by the caller
fn create_provider(provider_name: &str, model_config: ModelConfig) -> Result<Arc<dyn Provider>, ProviderError> {
    // Create the provider directly with parameters from environment variables
    // This avoids using Config::global() entirely
    match provider_name {
        "databricks" => {
            // Get host and token directly from environment variables
            let host = match std::env::var("DATABRICKS_HOST") {
                Ok(host) => host,
                Err(_) => return Err(ProviderError::RequestFailed(
                    "DATABRICKS_HOST environment variable not set".to_string()
                )),
            };
            
            let token = match std::env::var("DATABRICKS_TOKEN") {
                Ok(token) => token,
                Err(_) => return Err(ProviderError::RequestFailed(
                    "DATABRICKS_TOKEN environment variable not set".to_string()
                )),
            };
            
            match goose::providers::databricks::DatabricksProvider::from_params(host, token, model_config) {
                Ok(provider) => Ok(Arc::new(provider)),
                Err(e) => Err(ProviderError::RequestFailed(format!("Failed to create Databricks provider: {}", e))),
            }
        },
        // Add other providers as needed with their from_params implementations
        _ => {
            // For providers we haven't implemented direct parameter creation yet,
            // fall back to the factory but with a clear warning
            println!("Warning: Using Config::global() for provider {}", provider_name);
            match providers::create(provider_name, model_config) {
                Ok(provider) => Ok(provider),
                Err(e) => Err(ProviderError::RequestFailed(format!("Failed to create provider {}: {}", provider_name, e)))
            }
        }
    }
}
