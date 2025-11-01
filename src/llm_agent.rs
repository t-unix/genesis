use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Parser)]
#[command(name = "smart-home-llm")]
#[command(about = "LLM-powered smart home control agent", long_about = None)]
struct Cli {
    /// Natural language order (e.g., "turn on the kitchen lights")
    order: String,

    #[arg(long, env = "HOMEBRIDGE_URL", default_value = "http://192.168.178.67:8581")]
    homebridge_url: String,

    #[arg(long, env = "HOMEBRIDGE_USERNAME")]
    homebridge_username: Option<String>,

    #[arg(long, env = "HOMEBRIDGE_PASSWORD")]
    homebridge_password: Option<String>,

    #[arg(long, env = "ANTHROPIC_API_KEY")]
    anthropic_api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct Accessory {
    #[serde(rename = "uniqueId")]
    unique_id: String,
    #[serde(rename = "serviceName")]
    service_name: String,
    #[serde(rename = "type")]
    device_type: String,
    values: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct ControlRequest {
    #[serde(rename = "characteristicType")]
    characteristic_type: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    system: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct Action {
    device: String,
    action: String,
    #[serde(default)]
    brightness: Option<u8>,
}

struct SmartHomeAgent {
    client: Client,
    homebridge_url: String,
    token: String,
    devices: Vec<Accessory>,
}

impl SmartHomeAgent {
    fn new(homebridge_url: String, username: String, password: String) -> Result<Self> {
        let client = Client::new();

        // Authenticate
        println!("🔐 Authenticating with Homebridge...");
        let login_response: LoginResponse = client
            .post(format!("{}/api/auth/login", homebridge_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .context("Failed to authenticate")?
            .json()
            .context("Failed to parse login response")?;

        let token = login_response.access_token;

        // Discover devices
        println!("🔍 Discovering devices...");
        let devices: Vec<Accessory> = client
            .get(format!("{}/api/accessories", homebridge_url))
            .bearer_auth(&token)
            .send()
            .context("Failed to fetch accessories")?
            .json()
            .context("Failed to parse accessories")?;

        println!("✅ Found {} devices\n", devices.len());

        Ok(Self {
            client,
            homebridge_url,
            token,
            devices,
        })
    }

    fn get_device_list(&self) -> String {
        let controllable: Vec<String> = self
            .devices
            .iter()
            .filter(|d| matches!(d.device_type.as_str(), "Lightbulb" | "Switch" | "Outlet"))
            .map(|d| d.service_name.clone())
            .collect();

        controllable.join(", ")
    }

    fn find_device(&self, query: &str) -> Option<&Accessory> {
        let query_lower = query.to_lowercase();

        // Exact match first
        if let Some(device) = self
            .devices
            .iter()
            .find(|d| d.service_name.to_lowercase() == query_lower)
        {
            return Some(device);
        }

        // Partial match
        let matches: Vec<&Accessory> = self
            .devices
            .iter()
            .filter(|d| d.service_name.to_lowercase().contains(&query_lower))
            .collect();

        if matches.len() == 1 {
            Some(matches[0])
        } else {
            None
        }
    }

    fn control_device(
        &self,
        device_name: &str,
        characteristic: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        let device = self
            .find_device(device_name)
            .context(format!("Device not found: {}", device_name))?;

        let request = ControlRequest {
            characteristic_type: characteristic.to_string(),
            value,
        };

        self.client
            .put(format!(
                "{}/api/accessories/{}",
                self.homebridge_url, device.unique_id
            ))
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .context("Failed to control device")?;

        println!("✅ {}: {} = {}", device.service_name, characteristic, request.value);

        Ok(())
    }

    fn execute_action(&self, action: &Action) -> Result<()> {
        match action.action.as_str() {
            "on" => self.control_device(&action.device, "On", serde_json::json!(1)),
            "off" => self.control_device(&action.device, "On", serde_json::json!(0)),
            "brightness" => {
                if let Some(level) = action.brightness {
                    self.control_device(&action.device, "Brightness", serde_json::json!(level))
                } else {
                    anyhow::bail!("Brightness action requires brightness value")
                }
            }
            _ => anyhow::bail!("Unknown action: {}", action.action),
        }
    }
}

fn parse_order_with_claude(
    api_key: &str,
    order: &str,
    device_list: &str,
) -> Result<Vec<Action>> {
    let client = Client::new();

    let system_prompt = format!(
        r#"You are a smart home automation assistant. Your job is to parse natural language commands and convert them to JSON actions.

Available devices: {}

Return ONLY a JSON array of actions, with NO additional text. Each action must have:
- "device": exact device name from the list above (use partial matching if needed)
- "action": one of "on", "off", or "brightness"
- "brightness": optional number 0-100 (only for brightness action)

Examples:
Input: "turn on kitchen lights"
Output: [{{"device": "Kuechentisch Licht 1", "action": "on"}}, {{"device": "Kuechentisch Licht 2", "action": "on"}}]

Input: "set living room to 50%"
Output: [{{"device": "Wohnzimmer Deckenlampe", "action": "brightness", "brightness": 50}}]

Input: "lights off in office"
Output: [{{"device": "Arbeitszimmer Deckenlampe", "action": "off"}}]

Return ONLY valid JSON, nothing else."#,
        device_list
    );

    let request = ClaudeRequest {
        model: "claude-3-5-haiku-20241022".to_string(),
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: order.to_string(),
        }],
        system: system_prompt,
    };

    println!("🤖 Asking Claude Haiku to parse: \"{}\"", order);

    let response: ClaudeResponse = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .context("Failed to call Claude API")?
        .json()
        .context("Failed to parse Claude response")?;

    let text = response
        .content
        .first()
        .context("No content in Claude response")?
        .text
        .as_str();

    println!("📝 Claude response: {}\n", text);

    let actions: Vec<Action> =
        serde_json::from_str(text).context("Failed to parse actions from Claude response")?;

    Ok(actions)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get Anthropic API key
    let api_key = cli
        .anthropic_api_key
        .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
        .context("ANTHROPIC_API_KEY is required (via --anthropic-api-key or env var)")?;

    // Get Homebridge credentials
    let username = cli
        .homebridge_username
        .or_else(|| env::var("HOMEBRIDGE_USERNAME").ok())
        .context("HOMEBRIDGE_USERNAME is required (via --homebridge-username or env var)")?;

    let password = cli
        .homebridge_password
        .or_else(|| env::var("HOMEBRIDGE_PASSWORD").ok())
        .context("HOMEBRIDGE_PASSWORD is required (via --homebridge-password or env var)")?;

    println!("🏠 Smart Home LLM Agent\n");
    println!("📋 Order: {}\n", cli.order);

    // Initialize agent
    let agent = SmartHomeAgent::new(cli.homebridge_url, username, password)?;

    // Get device list
    let device_list = agent.get_device_list();

    // Parse order using Claude
    let actions = parse_order_with_claude(&api_key, &cli.order, &device_list)?;

    if actions.is_empty() {
        println!("⚠️  No actions to execute");
        return Ok(());
    }

    println!("🎯 Executing {} action(s)...\n", actions.len());

    // Execute actions
    for (i, action) in actions.iter().enumerate() {
        println!("[{}/{}] {:?}", i + 1, actions.len(), action);
        agent.execute_action(action)?;
    }

    println!("\n🎉 All actions completed successfully!");

    Ok(())
}
