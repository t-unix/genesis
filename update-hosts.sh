#!/bin/bash

# Update /etc/hosts with all ingress entries from a k3s cluster

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <cluster-name> <cluster-ip>"
  echo "Example: $0 squid 192.168.178.67"
  exit 1
fi

CLUSTER_NAME="$1"
CLUSTER_IP="$2"
KUBECONFIG_FILE="${HOME}/.kube/config-${CLUSTER_NAME}"
HOSTS_FILE="/etc/hosts"
MARKER_START="# BEGIN genesis-${CLUSTER_NAME}"
MARKER_END="# END genesis-${CLUSTER_NAME}"

# Check if kubeconfig exists
if [ ! -f "${KUBECONFIG_FILE}" ]; then
  echo "✗ Kubeconfig not found at ${KUBECONFIG_FILE}"
  echo "Run ./get-kubeconfig.sh first"
  exit 1
fi

export KUBECONFIG="${KUBECONFIG_FILE}"

echo "Fetching ingress entries from cluster: ${CLUSTER_NAME}"

# Get all ingress hostnames
HOSTNAMES=$(kubectl get ingress -A -o jsonpath='{range .items[*]}{.spec.rules[*].host}{"\n"}{end}' | sort -u)

if [ -z "$HOSTNAMES" ]; then
  echo "✗ No ingress entries found in cluster"
  exit 1
fi

echo "Found ingress entries:"
echo "$HOSTNAMES" | sed 's/^/  - /'
echo ""

# Create temporary file with new entries
TEMP_ENTRIES=$(mktemp)
echo "${MARKER_START}" > "${TEMP_ENTRIES}"
echo "$HOSTNAMES" | while read -r hostname; do
  if [ -n "$hostname" ]; then
    echo "${CLUSTER_IP} ${hostname}" >> "${TEMP_ENTRIES}"
  fi
done
echo "${MARKER_END}" >> "${TEMP_ENTRIES}"

# Create backup
BACKUP_FILE="${HOSTS_FILE}.backup-$(date +%Y%m%d-%H%M%S)"
echo "Creating backup: ${BACKUP_FILE}"
sudo cp "${HOSTS_FILE}" "${BACKUP_FILE}"

# Remove old entries between markers
TEMP_HOSTS=$(mktemp)
awk -v start="${MARKER_START}" -v end="${MARKER_END}" '
  $0 == start { skip=1; next }
  $0 == end { skip=0; next }
  !skip { print }
' "${HOSTS_FILE}" > "${TEMP_HOSTS}"

# Append new entries
cat "${TEMP_ENTRIES}" >> "${TEMP_HOSTS}"

# Update hosts file
sudo cp "${TEMP_HOSTS}" "${HOSTS_FILE}"

# Cleanup
rm -f "${TEMP_ENTRIES}" "${TEMP_HOSTS}"

echo ""
echo "✓ Hosts file updated successfully"
echo ""
echo "Added entries:"
grep -A 100 "${MARKER_START}" "${HOSTS_FILE}" | grep -B 100 "${MARKER_END}" | grep -v "^#"
