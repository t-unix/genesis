use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

#[derive(Parser)]
#[command(name = "smart-home-agent")]
#[command(about = "Control your smart home devices via Homebridge API", long_about = None)]
struct Cli {
    #[arg(long, default_value = "http://192.168.178.67:8581")]
    url: String,

    #[arg(long)]
    username: Option<String>,

    #[arg(long)]
    password: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available devices
    List,

    /// Turn device on
    On {
        /// Device name (partial match supported)
        device: String,
    },

    /// Turn device off
    Off {
        /// Device name (partial match supported)
        device: String,
    },

    /// Set device brightness
    Brightness {
        /// Device name
        device: String,
        /// Brightness level (0-100)
        level: u8,
    },

    /// Control kitchen lights
    Kitchen {
        /// State: on, off, ein, aus
        state: String,
    },
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
    #[serde(rename = "humanType")]
    human_type: Option<String>,
    values: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct ControlRequest {
    #[serde(rename = "characteristicType")]
    characteristic_type: String,
    value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct K8sSecret {
    data: HashMap<String, String>,
}

struct SmartHomeAgent {
    client: Client,
    base_url: String,
    token: String,
    devices: Vec<Accessory>,
}

impl SmartHomeAgent {
    fn new(base_url: String, username: String, password: String) -> Result<Self> {
        let client = Client::new();

        // Authenticate
        println!("🔐 Authenticating...");
        let login_response: LoginResponse = client
            .post(format!("{}/api/auth/login", base_url))
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
        println!("🔍 Discovering devices...\n");
        let devices: Vec<Accessory> = client
            .get(format!("{}/api/accessories", base_url))
            .bearer_auth(&token)
            .send()
            .context("Failed to fetch accessories")?
            .json()
            .context("Failed to parse accessories")?;

        Ok(Self {
            client,
            base_url,
            token,
            devices,
        })
    }

    fn list_devices(&self) {
        println!("🏠 Available Devices:");
        println!("{}", "=".repeat(60));

        for device in &self.devices {
            // Filter to only show controllable devices
            if !matches!(
                device.device_type.as_str(),
                "Lightbulb" | "Switch" | "Outlet"
            ) {
                continue;
            }

            let mut status = String::new();

            if let Some(values) = &device.values {
                if let Some(on_value) = values.get("On") {
                    status.push_str(if on_value.as_i64() == Some(1) {
                        "[ON]"
                    } else {
                        "[OFF]"
                    });
                }

                if let Some(brightness) = values.get("Brightness") {
                    if let Some(b) = brightness.as_i64() {
                        status.push_str(&format!(" {}%", b));
                    }
                }
            }

            let human_type = device.human_type.as_deref().unwrap_or("Unknown");

            println!(
                "  • {:<35} {:<15} {}",
                device.service_name, human_type, status
            );
        }
        println!();
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

        match matches.len() {
            0 => None,
            1 => Some(matches[0]),
            _ => {
                println!("⚠️  Multiple devices match '{}':", query);
                for device in matches {
                    println!("  • {}", device.service_name);
                }
                None
            }
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
                self.base_url, device.unique_id
            ))
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .context("Failed to control device")?;

        println!("✅ {}: {} = {}", device.service_name, characteristic, request.value);

        Ok(())
    }

    fn turn_on(&self, device_name: &str) -> Result<()> {
        self.control_device(device_name, "On", serde_json::json!(1))
    }

    fn turn_off(&self, device_name: &str) -> Result<()> {
        self.control_device(device_name, "On", serde_json::json!(0))
    }

    fn set_brightness(&self, device_name: &str, brightness: u8) -> Result<()> {
        let brightness = brightness.min(100);
        self.control_device(device_name, "Brightness", serde_json::json!(brightness))
    }

    fn kitchen_lights(&self, state: &str) -> Result<()> {
        let lights = ["kuechentisch licht 1", "kuechentisch licht 2"];
        let is_on = matches!(state.to_lowercase().as_str(), "on" | "ein" | "1" | "true");

        for light in lights {
            if is_on {
                self.turn_on(light)?;
            } else {
                self.turn_off(light)?;
            }
        }

        Ok(())
    }
}

fn get_credentials_from_k8s() -> Result<(String, String)> {
    let output = Command::new("kubectl")
        .args([
            "--kubeconfig=/Users/flx/.kube/config-squid",
            "get",
            "secret",
            "homebridge-credentials",
            "-n",
            "default",
            "-o",
            "json",
        ])
        .output()
        .context("Failed to execute kubectl")?;

    if !output.status.success() {
        anyhow::bail!("kubectl command failed");
    }

    let secret: K8sSecret = serde_json::from_slice(&output.stdout)
        .context("Failed to parse kubectl output")?;

    let username = secret
        .data
        .get("username")
        .context("username not found in secret")?;
    let password = secret
        .data
        .get("password")
        .context("password not found in secret")?;

    let username = String::from_utf8(
        base64::decode(username).context("Failed to decode username")?,
    )
    .context("Invalid UTF-8 in username")?;

    let password = String::from_utf8(
        base64::decode(password).context("Failed to decode password")?,
    )
    .context("Invalid UTF-8 in password")?;

    Ok((username, password))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get credentials
    let (username, password) = if let (Some(u), Some(p)) = (cli.username, cli.password) {
        (u, p)
    } else {
        println!("📦 Loading credentials from k8s secret...");
        get_credentials_from_k8s()
            .context("Failed to load credentials. Use --username and --password flags")?
    };

    // Initialize agent
    let agent = SmartHomeAgent::new(cli.url, username, password)?;

    // Execute command
    match cli.command {
        Commands::List => agent.list_devices(),
        Commands::On { device } => agent.turn_on(&device)?,
        Commands::Off { device } => agent.turn_off(&device)?,
        Commands::Brightness { device, level } => agent.set_brightness(&device, level)?,
        Commands::Kitchen { state } => agent.kitchen_lights(&state)?,
    }

    Ok(())
}
