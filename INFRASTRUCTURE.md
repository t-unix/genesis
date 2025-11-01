# Genesis Infrastructure Documentation

## Cluster Information

### Squid k3s Cluster
- **Hostname**: squid
- **IP Address**: 192.168.178.67
- **k3s Version**: v1.33.5+k3s1
- **Kubeconfig**: `~/.kube/config-squid`

### Boot Configuration
Memory cgroups enabled in `/boot/cmdline.txt`:
```
cgroup_memory=1 cgroup_enable=memory
```

## Management Scripts

### get-kubeconfig.sh
Fetches kubeconfig from k3s cluster and updates API server IP.

**Usage:**
```bash
./get-kubeconfig.sh <cluster-name> <cluster-ip>
```

**Example:**
```bash
./get-kubeconfig.sh squid 192.168.178.67
```

**Output**: `~/.kube/config-squid` (permissions: 600)

### deploy-argocd.sh
Deploys ArgoCD on k3s cluster using Helm with ingress.

**Usage:**
```bash
./deploy-argocd.sh <cluster-name>
```

**Example:**
```bash
./deploy-argocd.sh squid
```

**Configuration**: Uses `gitops/argocd/values.yaml` and `gitops/argocd/ingress.yaml`

### update-hosts.sh
Updates `/etc/hosts` with all ingress entries from the cluster.

**Usage:**
```bash
./update-hosts.sh <cluster-name> <cluster-ip>
```

**Example:**
```bash
./update-hosts.sh squid 192.168.178.67
```

**Note**: Requires sudo. Creates automatic backups and uses markers for safe updates.

## ArgoCD

### Access
- **URL**: http://argocd.genesis
- **Port Forward**: `kubectl port-forward svc/argocd-server -n argocd 8080:80`
- **Username**: admin
- **Password**: `kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath='{.data.password}' | base64 -d`

### Configuration
- **Namespace**: argocd
- **Helm Chart**: argo/argo-cd v7.7.11
- **Ingress**: argocd.genesis
- **Settings**:
  - `server.insecure: true` (for ingress compatibility)
  - `server.service.type: LoadBalancer`

## Homebridge API

### Access
- **URL**: http://192.168.178.67:8581
- **Swagger Docs**: http://192.168.178.67:8581/swagger
- **Process**: Running as systemd service on squid (PID 1779)
- **UI Port**: 8581 (hb-service)

### Authentication
Credentials stored in Kubernetes secret: `homebridge-credentials` (namespace: default)

**Get Token:**
```bash
curl -X POST http://192.168.178.67:8581/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"USERNAME","password":"PASSWORD"}' \
  | jq -r '.access_token'
```

### API Endpoints
- `POST /api/auth/login` - Authenticate and get token
- `GET /api/accessories` - List all HomeKit accessories
- `GET /api/accessories/{uniqueId}` - Get specific accessory
- `PUT /api/accessories/{uniqueId}` - Control accessory

### Control Lights Example
```bash
# Turn light ON
curl -X PUT http://192.168.178.67:8581/api/accessories/{uniqueId} \
  -H "Authorization: Bearer {TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"characteristicType":"On","value":1}'

# Turn light OFF
curl -X PUT http://192.168.178.67:8581/api/accessories/{uniqueId} \
  -H "Authorization: Bearer {TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"characteristicType":"On","value":0}'
```

### Kitchen Lights
- **Kuechentisch Licht 1**
  - Unique ID: `b6d08f1f799f39daf155a9dea850d422854f4704c7c0ca23a11ec807e5fa8214`
  - Manufacturer: OSRAM
  - Model: AA70155 (Classic A60 TW)
  - Serial: 0x8418260000c9790f

- **Kuechentisch Licht 2**
  - Unique ID: `289588bce15e93d0ecd905bd6ffce5dbfcc2f445f29bc6d60de82e6c6e09eafe`
  - Manufacturer: OSRAM
  - Model: AA70155 (Classic A60 TW)
  - Serial: 0x8418260000d9a574

## Zigbee2MQTT

### Access
- **Web UI**: http://192.168.178.67:8080
- **MQTT Broker**: 192.168.178.67:1883
- **Docker Container**: `zigbee2mqtt`
- **MQTT Container**: `zigbee2mqtt-mqtt-1` (eclipse-mosquitto:2.0)

### MQTT Control

**Zigbee2MQTT uses MQTT exclusively - no REST API available.**

#### Control Devices
```bash
# Turn light OFF
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub \
  -h localhost \
  -t 'zigbee2mqtt/Kuechentisch Licht 1/set' \
  -m '{\"state\":\"OFF\"}'"

# Turn light ON
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub \
  -h localhost \
  -t 'zigbee2mqtt/Kuechentisch Licht 1/set' \
  -m '{\"state\":\"ON\"}'"
```

#### MQTT Topics
- `zigbee2mqtt/FRIENDLY_NAME/set` - Control devices
- `zigbee2mqtt/FRIENDLY_NAME` - Device state updates
- `zigbee2mqtt/FRIENDLY_NAME/get` - Read device values
- `zigbee2mqtt/bridge/devices` - List all devices
- `zigbee2mqtt/bridge/info` - System information

#### Get Device List
```bash
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_sub \
  -h localhost \
  -t 'zigbee2mqtt/bridge/devices' \
  -C 1"
```

## Kubernetes Secrets

### homebridge-credentials
- **Namespace**: default
- **Keys**:
  - `username` - Homebridge username
  - `password` - Homebridge password
  - `url` - Homebridge URL (http://192.168.178.67:8581)

**Access:**
```bash
kubectl --kubeconfig=/Users/flx/.kube/config-squid \
  get secret homebridge-credentials -n default \
  -o json | jq -r '.data | map_values(@base64d)'
```

## Network Configuration

### Ingress Entries
All ingress entries are managed via Traefik and can be updated in `/etc/hosts` using `update-hosts.sh`.

Current ingresses:
- `argocd.genesis` → 192.168.178.67

### Port Mappings
- **1883** - Mosquitto MQTT
- **6443** - k3s API server
- **8080** - Zigbee2MQTT Web UI
- **8581** - Homebridge UI
- **9001** - Mosquitto WebSocket

## GitOps Structure

```
gitops/
└── argocd/
    ├── values.yaml    # Helm values for ArgoCD
    └── ingress.yaml   # Ingress configuration
```

## Common Commands

### Kubernetes
```bash
# Export kubeconfig
export KUBECONFIG=~/.kube/config-squid

# Get all pods
kubectl get pods -A

# Get ingresses
kubectl get ingress -A

# Get nodes
kubectl get nodes
```

### SSH to Squid
```bash
ssh squid
```

### Smart Home Agent (Recommended)
```bash
# Build the agent
cargo build --release

# List all devices
./target/release/smart-home-agent list

# Control kitchen lights (quick)
./target/release/smart-home-agent kitchen ein
./target/release/smart-home-agent kitchen aus

# Control any device by name
./target/release/smart-home-agent on "wohnzimmer"
./target/release/smart-home-agent off "arbeitszimmer"

# Set brightness
./target/release/smart-home-agent brightness "bruno schrank" 50
```

See [README-AGENT.md](README-AGENT.md) for full documentation.

### Control Kitchen Lights (MQTT Alternative)
```bash
# Off
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub -h localhost -t 'zigbee2mqtt/Kuechentisch Licht 1/set' -m '{\"state\":\"OFF\"}'"
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub -h localhost -t 'zigbee2mqtt/Kuechentisch Licht 2/set' -m '{\"state\":\"OFF\"}'"

# On
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub -h localhost -t 'zigbee2mqtt/Kuechentisch Licht 1/set' -m '{\"state\":\"ON\"}'"
ssh squid "docker exec zigbee2mqtt-mqtt-1 mosquitto_pub -h localhost -t 'zigbee2mqtt/Kuechentisch Licht 2/set' -m '{\"state\":\"ON\"}'"
```
