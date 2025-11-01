# Smart Home LLM Agent 🤖🏠

An intelligent, LLM-powered smart home control agent that understands natural language commands and runs as ephemeral Kubernetes jobs.

## Overview

This agent uses **Claude 3.5 Haiku** to parse natural language orders and translate them into smart home actions. Each command runs as a separate Kubernetes Job, making it stateless, scalable, and cost-effective.

### Architecture

```
Natural Language Order
        ↓
 Kubernetes Job Created
        ↓
  Container Starts
        ↓
Claude Haiku Parses Intent
        ↓
 Homebridge API Called
        ↓
  Devices Controlled
        ↓
  Job Completes & Exits
```

## Features

- 🧠 **AI-Powered** - Uses Claude 3.5 Haiku for natural language understanding
- ⚡ **Fast** - Haiku model provides sub-second responses
- 💰 **Cost-Effective** - Only runs when needed, Haiku is very cheap
- 🔒 **Secure** - Credentials stored in Kubernetes secrets
- 📦 **Containerized** - Runs in Docker containers on Kubernetes
- 🚀 **Auto-Built** - GitHub Actions automatically builds and pushes images
- 🔄 **Stateless** - Each job is independent and self-contained

## Prerequisites

1. **Kubernetes cluster** (k3s on squid)
2. **Anthropic API key** (for Claude Haiku)
3. **Homebridge** running and accessible
4. **kubectl** configured with cluster access

## Setup

### 1. Create Anthropic API Secret

```bash
kubectl --kubeconfig=~/.kube/config-squid create secret generic anthropic-credentials \
  --from-literal=api-key=YOUR_ANTHROPIC_API_KEY \
  -n default
```

Get your API key from: https://console.anthropic.com/

### 2. Verify Homebridge Secret Exists

```bash
kubectl --kubeconfig=~/.kube/config-squid get secret homebridge-credentials -n default
```

If it doesn't exist, create it:
```bash
kubectl --kubeconfig=~/.kube/config-squid create secret generic homebridge-credentials \
  --from-literal=username=YOUR_USERNAME \
  --from-literal=password=YOUR_PASSWORD \
  -n default
```

### 3. Update Job Manifest

Edit `k8s/smart-home-llm-job.yaml` and replace `YOUR_GITHUB_USERNAME` with your actual GitHub username.

### 4. Build and Push Container (via GitHub Actions)

The container is automatically built when you push to GitHub:

```bash
git add .
git commit -m "Add LLM smart home agent"
git push
```

GitHub Actions will build and push to `ghcr.io/YOUR_USERNAME/genesis/smart-home-llm:latest`

**Or build locally:**
```bash
docker build -t ghcr.io/YOUR_USERNAME/genesis/smart-home-llm:latest .
docker push ghcr.io/YOUR_USERNAME/genesis/smart-home-llm:latest
```

## Usage

### Easy Way: Helper Script

```bash
./run-smart-home-order.sh "turn on the kitchen lights"
```

### Examples

```bash
# Turn on lights
./run-smart-home-order.sh "turn on kitchen lights"

# Turn off lights
./run-smart-home-order.sh "lights off in the office"

# Set brightness
./run-smart-home-order.sh "set living room to 50%"

# Multiple actions
./run-smart-home-order.sh "turn on bedroom and set it to 30%"

# Complex commands
./run-smart-home-order.sh "make the kitchen bright and turn off the office"
```

### Manual Job Creation

```bash
# Create job with custom order
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
sed "s/TIMESTAMP/${TIMESTAMP}/g; s/ORDER_PLACEHOLDER/turn on kitchen lights/g" \
  k8s/smart-home-llm-job.yaml | kubectl apply -f -

# Watch job progress
kubectl logs -f job/smart-home-llm-${TIMESTAMP}

# Check status
kubectl get job smart-home-llm-${TIMESTAMP}
```

## How It Works

### 1. Job Creation
When you run a command, a new Kubernetes Job is created with:
- Unique timestamp-based name
- Your natural language order as an argument
- Credentials from Kubernetes secrets

### 2. Claude Haiku Processing
The agent:
1. Connects to Homebridge and discovers all devices
2. Sends your order + device list to Claude Haiku
3. Receives structured JSON actions (device, action, brightness)

Example Claude response for "turn on kitchen lights":
```json
[
  {"device": "Kuechentisch Licht 1", "action": "on"},
  {"device": "Kuechentisch Licht 2", "action": "on"}
]
```

### 3. Action Execution
The agent:
1. Finds each device in the Homebridge inventory
2. Executes the action via Homebridge API
3. Reports success or failure

### 4. Job Completion
The Job completes and is automatically cleaned up after 1 hour (configurable via `ttlSecondsAfterFinished`).

## Command Examples

| Natural Language | Parsed Actions |
|-----------------|----------------|
| "turn on kitchen lights" | Turn on Kuechentisch Licht 1 & 2 |
| "set living room to 50%" | Set Wohnzimmer Deckenlampe brightness to 50 |
| "lights off in office" | Turn off Arbeitszimmer Deckenlampe |
| "make bedroom dark" | Set bedroom brightness to 0 or turn off |
| "brighten the kitchen" | Set kitchen lights to 100% |

## Monitoring

### View Running Jobs
```bash
kubectl get jobs -l app=smart-home-llm
```

### View Job Logs
```bash
kubectl logs job/smart-home-llm-TIMESTAMP
```

### View All Job History
```bash
kubectl get jobs --sort-by=.metadata.creationTimestamp
```

### Delete Old Jobs
Jobs are auto-deleted after 1 hour, but you can manually clean up:
```bash
kubectl delete job -l app=smart-home-llm
```

## Configuration

### Environment Variables (in Job manifest)

| Variable | Source | Description |
|----------|--------|-------------|
| `HOMEBRIDGE_URL` | Direct | Homebridge API endpoint |
| `HOMEBRIDGE_USERNAME` | Secret | Homebridge username |
| `HOMEBRIDGE_PASSWORD` | Secret | Homebridge password |
| `ANTHROPIC_API_KEY` | Secret | Claude API key |

### Resource Limits

Default limits (configurable in `k8s/smart-home-llm-job.yaml`):
- Memory: 128Mi (request) / 256Mi (limit)
- CPU: 100m (request) / 500m (limit)

### TTL (Time To Live)

Jobs are kept for 1 hour after completion. Change `ttlSecondsAfterFinished` to adjust.

## Troubleshooting

### "Secret not found"
Ensure secrets exist:
```bash
kubectl get secret homebridge-credentials -n default
kubectl get secret anthropic-credentials -n default
```

### "Image pull failed"
Check image name in Job manifest matches your GitHub username.

Make sure the image is public or add image pull secrets.

### "Job failed"
View logs:
```bash
kubectl logs job/smart-home-llm-TIMESTAMP
```

Common issues:
- Anthropic API key invalid/expired
- Homebridge not accessible from cluster
- Device name not found in Homebridge

### "Claude returns invalid JSON"
The agent expects Claude to return pure JSON. If Claude adds explanatory text, it will fail. The system prompt is designed to prevent this, but you can debug by checking logs.

## Cost Estimation

**Claude 3.5 Haiku Pricing** (as of 2025):
- Input: $0.80 per million tokens (~$0.0008 per request)
- Output: $4.00 per million tokens (~$0.004 per request)

**Typical request**: ~$0.005 (half a cent)

**100 commands/day** = ~$0.50/day = ~$15/month

Very cost-effective for smart home automation! 💰

## Development

### Build Locally
```bash
cargo build --release --bin smart-home-llm
```

### Test Locally (without Kubernetes)
```bash
export HOMEBRIDGE_URL=http://192.168.178.67:8581
export HOMEBRIDGE_USERNAME=your_username
export HOMEBRIDGE_PASSWORD=your_password
export ANTHROPIC_API_KEY=your_api_key

./target/release/smart-home-llm "turn on kitchen lights"
```

### Build Docker Image
```bash
docker build -t smart-home-llm:test .
docker run --rm \
  -e HOMEBRIDGE_URL=http://192.168.178.67:8581 \
  -e HOMEBRIDGE_USERNAME=user \
  -e HOMEBRIDGE_PASSWORD=pass \
  -e ANTHROPIC_API_KEY=key \
  smart-home-llm:test "turn on kitchen lights"
```

## GitHub Actions

The workflow (`.github/workflows/build-llm-agent.yml`) automatically:
1. Triggers on push to `main` or PR
2. Builds the Docker image
3. Pushes to GitHub Container Registry (ghcr.io)
4. Tags with `latest` and `main-SHA`

## Future Enhancements

- [ ] Add support for scenes/routines
- [ ] Temperature sensor monitoring
- [ ] Scheduling (via CronJobs)
- [ ] Slack/Discord integration
- [ ] Voice input support
- [ ] Multi-room awareness
- [ ] Learning user preferences

## License

Part of the Genesis infrastructure project.
