#!/bin/bash

# Deploy ArgoCD on k3s cluster using Helm

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <cluster-name>"
  echo "Example: $0 squid"
  exit 1
fi

CLUSTER_NAME="$1"
KUBECONFIG_FILE="${HOME}/.kube/config-${CLUSTER_NAME}"
ARGOCD_NAMESPACE="argocd"

# Check if kubeconfig exists
if [ ! -f "${KUBECONFIG_FILE}" ]; then
  echo "✗ Kubeconfig not found at ${KUBECONFIG_FILE}"
  echo "Run ./get-kubeconfig.sh first"
  exit 1
fi

export KUBECONFIG="${KUBECONFIG_FILE}"

echo "Deploying ArgoCD on cluster: ${CLUSTER_NAME}"
echo "Using kubeconfig: ${KUBECONFIG_FILE}"
echo ""

# Add ArgoCD Helm repository
echo "Adding ArgoCD Helm repository..."
helm repo add argo https://argoproj.github.io/argo-helm
helm repo update

# Create namespace
echo "Creating namespace: ${ARGOCD_NAMESPACE}..."
kubectl create namespace ${ARGOCD_NAMESPACE} --dry-run=client -o yaml | kubectl apply -f -

# Install ArgoCD
echo "Installing ArgoCD..."
helm install argocd argo/argo-cd \
  --namespace ${ARGOCD_NAMESPACE} \
  --version 7.7.11 \
  --values gitops/argocd/values.yaml

if [ $? -eq 0 ]; then
  echo ""
  echo "✓ ArgoCD deployed successfully"
  echo ""
  echo "Waiting for ArgoCD to be ready..."
  kubectl wait --for=condition=available --timeout=300s \
    deployment/argocd-server -n ${ARGOCD_NAMESPACE}

  echo ""
  echo "Applying ArgoCD ingress..."
  kubectl apply -f gitops/argocd/ingress.yaml

  echo ""
  echo "✓ ArgoCD ingress created: argocd.genesis"
  echo ""
  echo "Get initial admin password:"
  echo "  kubectl -n ${ARGOCD_NAMESPACE} get secret argocd-initial-admin-secret -o jsonpath='{.data.password}' | base64 -d"
  echo ""
  echo "To update /etc/hosts with ingress entries, run:"
  echo "  ./update-hosts.sh ${CLUSTER_NAME} <cluster-ip>"
  echo ""
  echo "Access ArgoCD at: http://argocd.genesis"
  echo "Username: admin"
  echo ""
  echo "Or use port-forward:"
  echo "  kubectl port-forward svc/argocd-server -n ${ARGOCD_NAMESPACE} 8080:80"
  echo "  http://localhost:8080"
else
  echo "✗ Failed to deploy ArgoCD"
  exit 1
fi
