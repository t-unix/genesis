#!/bin/bash

# Run Smart Home LLM Agent as Kubernetes Job
# Usage: ./run-smart-home-order.sh "turn on kitchen lights"

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 \"<natural language order>\""
  echo ""
  echo "Examples:"
  echo "  $0 \"turn on kitchen lights\""
  echo "  $0 \"set living room to 50%\""
  echo "  $0 \"turn off all lights in the office\""
  exit 1
fi

ORDER="$1"
CLUSTER_NAME="${2:-squid}"
KUBECONFIG_FILE="${HOME}/.kube/config-${CLUSTER_NAME}"

# Check if kubeconfig exists
if [ ! -f "${KUBECONFIG_FILE}" ]; then
  echo "✗ Kubeconfig not found at ${KUBECONFIG_FILE}"
  echo "Run ./get-kubeconfig.sh first"
  exit 1
fi

export KUBECONFIG="${KUBECONFIG_FILE}"

# Generate unique job name with timestamp
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
JOB_NAME="smart-home-llm-${TIMESTAMP}"

echo "🏠 Smart Home LLM Job Runner"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Order: ${ORDER}"
echo "🔖 Job: ${JOB_NAME}"
echo ""

# Check if anthropic secret exists
if ! kubectl get secret anthropic-credentials -n default &>/dev/null; then
  echo "⚠️  Warning: anthropic-credentials secret not found"
  echo ""
  echo "To create it, run:"
  echo "  kubectl create secret generic anthropic-credentials \\"
  echo "    --from-literal=api-key=YOUR_ANTHROPIC_API_KEY \\"
  echo "    -n default"
  echo ""
  exit 1
fi

# Create temporary job manifest
TEMP_JOB=$(mktemp)
trap "rm -f ${TEMP_JOB}" EXIT

# Replace placeholders in job template
sed "s/TIMESTAMP/${TIMESTAMP}/g; s/ORDER_PLACEHOLDER/${ORDER}/g" k8s/smart-home-llm-job.yaml > "${TEMP_JOB}"

# Apply the job
echo "🚀 Creating job..."
kubectl apply -f "${TEMP_JOB}"

if [ $? -ne 0 ]; then
  echo "✗ Failed to create job"
  exit 1
fi

echo ""
echo "⏳ Waiting for job to complete..."
kubectl wait --for=condition=complete --timeout=120s job/${JOB_NAME} 2>/dev/null || \
kubectl wait --for=condition=failed --timeout=120s job/${JOB_NAME} 2>/dev/null

echo ""
echo "📄 Job logs:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
kubectl logs job/${JOB_NAME}

# Check job status
JOB_STATUS=$(kubectl get job ${JOB_NAME} -o jsonpath='{.status.conditions[?(@.type=="Complete")].status}')

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ "${JOB_STATUS}" = "True" ]; then
  echo "✅ Job completed successfully"
else
  echo "❌ Job failed"
  exit 1
fi

echo ""
echo "To view job details:"
echo "  kubectl describe job ${JOB_NAME}"
echo ""
echo "To delete job manually:"
echo "  kubectl delete job ${JOB_NAME}"
