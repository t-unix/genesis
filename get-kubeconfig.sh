#!/bin/bash

# Fetch kubeconfig from k3s cluster and update API server IP

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <cluster-name> <cluster-ip>"
  echo "Example: $0 squid 192.168.178.67"
  exit 1
fi

CLUSTER_NAME="$1"
CLUSTER_IP="$2"
KUBECONFIG_FILE="${HOME}/.kube/config-${CLUSTER_NAME}"

# Create .kube directory if it doesn't exist
mkdir -p "${HOME}/.kube"

echo "Fetching kubeconfig from ${CLUSTER_NAME}..."
ssh ${CLUSTER_NAME} "sudo cat /etc/rancher/k3s/k3s.yaml" | \
  sed "s/127.0.0.1/${CLUSTER_IP}/g" > "${KUBECONFIG_FILE}"

if [ $? -eq 0 ]; then
  chmod 600 "${KUBECONFIG_FILE}"
  echo "✓ Kubeconfig saved to ${KUBECONFIG_FILE}"
  echo ""
  echo "To use this config, run:"
  echo "  export KUBECONFIG=${KUBECONFIG_FILE}"
  echo ""
  echo "Or test with:"
  echo "  kubectl --kubeconfig=${KUBECONFIG_FILE} get nodes"
else
  echo "✗ Failed to fetch kubeconfig"
  exit 1
fi
