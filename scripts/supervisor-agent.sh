#!/usr/bin/env bash
# crabjar-supervisor: Node-level deployment agent for crabjar conductor.
# Polls conductor for pending deployments, creates Podman sandboxes,
# injects the crabjar binary, starts agents, and reports status back.
#
# Environment variables:
#   CONDUCTOR_URL    - Base URL of the conductor (default: http://jambo:8091)
#   SUPERVISOR_ID    - Unique ID for this supervisor node (default: hostname)
#   CRABJAR_BINARY   - Path to the crabjar binary on this host
#   WORKSPACE_ROOT   - Root directory for deployment workspaces
#   POLL_INTERVAL    - Seconds between polls (default: 10)
#   MAX_CONCURRENT   - Max concurrent sandboxes (default: 2)

set -euo pipefail

CONDUCTOR_URL="${CONDUCTOR_URL:-http://jambo:8091}"
SUPERVISOR_ID="${SUPERVISOR_ID:-$(hostname)}"
CRABJAR_BINARY="${CRABJAR_BINARY:-$HOME/projects/active/crabjar/target/release/crabjar}"
WORKSPACE_ROOT="${WORKSPACE_ROOT:-$HOME/crabjar-deployments}"
POLL_INTERVAL="${POLL_INTERVAL:-10}"
MAX_CONCURRENT="${MAX_CONCURRENT:-2}"

log() {
    echo "[$(date -Iseconds)] [$SUPERVISOR_ID] $*" >&2
}

# Report deployment status back to conductor
report_status() {
    local deployment_id="$1"
    local status="$2"
    local sandbox_id="${3:-}"
    local error="${4:-}"
    local outcome="${5:-}"

    local payload="{\"status\":\"$status\""
    [ -n "$sandbox_id" ] && payload="$payload,\"sandbox_id\":\"$sandbox_id\""
    [ -n "$error" ] && payload="$payload,\"error\":\"$error\""
    if [ -n "$outcome" ]; then
        payload="$payload,\"outcome\":$(echo "$outcome")"
    fi
    payload="$payload}"

    local response http_code
    http_code=$(curl -s -o /tmp/status-response.json -w "%{http_code}" -X POST \
        "${CONDUCTOR_URL}/workers/deployments/${deployment_id}/status" \
        -H "Content-Type: application/json" \
        -d "$payload")
    response=$(cat /tmp/status-response.json 2>/dev/null || echo "")
    
    if [ "$http_code" != "200" ]; then
        log "WARN: Status update failed (HTTP $http_code) for $deployment_id: $response"
        return 1
    fi
    log "Reported status '$status' for deployment $deployment_id"
}

# Create and run a Podman sandbox for a deployment
run_deployment() {
    local deployment_id="$1"
    local task_spec_json="$2"
    local container_name="cj-deploy-${deployment_id:0:8}-$$"
    local workspace_dir="${WORKSPACE_ROOT}/${deployment_id}"

    log "Starting deployment $deployment_id in container $container_name"

    # Create workspace directory
    mkdir -p "$workspace_dir/workspace"
    mkdir -p "$workspace_dir/config"

    # Write task spec to workspace
    echo "$task_spec_json" > "$workspace_dir/task-spec.json"

    # Extract fields from task spec for environment
    local task_cpu task_memory timeout_s task_image task_command
    task_cpu=$(echo "$task_spec_json" | jq -r '.resources.cpu // 2' 2>/dev/null || echo "2")
    task_memory=$(echo "$task_spec_json" | jq -r '.resources.memory_mb // 4096' 2>/dev/null || echo "4096")
    timeout_s=$(echo "$task_spec_json" | jq -r '.timeout_seconds // 3600' 2>/dev/null || echo "3600")
    task_image=$(echo "$task_spec_json" | jq -r '.image // "alpine:3.20"' 2>/dev/null || echo "alpine:3.20")
    task_command=$(echo "$task_spec_json" | jq -r '.command // ""' 2>/dev/null || echo "")

    # Mount the target repo directly into the container (not via symlink)
    local repo_name
    repo_name=$(echo "$task_spec_json" | jq -r '.repo')
    local repo_path="/home/crombo/projects/active/$repo_name"
    local repo_mount_arg=""
    if [ -d "$repo_path" ] && [ "$(ls -A "$repo_path" 2>/dev/null)" ]; then
        repo_mount_arg="--mount type=bind,src=$repo_path,dst=/workspace/$repo_name,ro"
    fi

    log "Task: cpu=$task_cpu memory=${task_memory}MB timeout=${timeout_s}s repo=$repo_name image=$task_image"

    # Report running status
    report_status "$deployment_id" "running" "$container_name" || return 1

    # Run the task in a Podman container
    local exit_code=0
    if [ -n "$task_command" ]; then
        # Execute the specified command
        log "Executing custom command for $deployment_id"
        podman run --rm \
            --name "$container_name" \
            --network host \
            --memory "${task_memory}m" \
            --cpus "$task_cpu" \
            -v "$workspace_dir/workspace:/workspace" \
            $repo_mount_arg \
            -v "$workspace_dir/config:/config" \
            -v "$workspace_dir/task-spec.json:/task-spec.json:ro" \
            -e "CRABJAR_TASK_SPEC=/task-spec.json" \
            -e "CRABJAR_DEPLOYMENT_ID=$deployment_id" \
            -w /workspace \
            "$task_image" \
            sh -c "$task_command" 2>&1 | tee "$workspace_dir/agent.log" || exit_code=$?
    else
        # Default: basic repo validation (backward compatible)
        log "Running default validation for $deployment_id"
        podman run --rm \
            --name "$container_name" \
            --network host \
            --memory "${task_memory}m" \
            --cpus "$task_cpu" \
            -v "$workspace_dir/workspace:/workspace" \
            $repo_mount_arg \
            -v "$workspace_dir/config:/config" \
            -v "$workspace_dir/task-spec.json:/task-spec.json:ro" \
            -e "CRABJAR_TASK_SPEC=/task-spec.json" \
            -e "CRABJAR_DEPLOYMENT_ID=$deployment_id" \
            -w /workspace \
            "$task_image" \
            sh -c '
                set -e
                apk add --no-cache git jq > /dev/null 2>&1 || true
                
                TASK_ID=$(cat /task-spec.json | jq -r ".id")
                REPO=$(cat /task-spec.json | jq -r ".repo")
                TITLE=$(cat /task-spec.json | jq -r ".title")
                
                echo "=== Agent starting ==="
                echo "Task: $TASK_ID - $TITLE"
                echo "Repo: $REPO"
                
                if [ -d "/workspace/$REPO" ]; then
                    cd "/workspace/$REPO"
                    git status --short | head -5
                    echo "Repo verified: $(git rev-parse --short HEAD)"
                fi
                
                cat > /workspace/outcome.json << EOF
                {
                    "task_id": "$TASK_ID",
                    "status": "completed",
                    "repo": "$REPO",
                    "message": "Sandbox execution verified.",
                    "timestamp": "$(date -Iseconds)"
                }
                EOF
                
                echo "=== Agent completed ==="
            ' 2>&1 | tee "$workspace_dir/agent.log" || exit_code=$?
    fi

    local outcome_json="null"
    if [ $exit_code -eq 0 ]; then
        log "Deployment $deployment_id completed successfully"
        # Send outcome as a simple JSON string (no escaping issues)
        if [ -f "$workspace_dir/workspace/outcome.json" ]; then
            log "Outcome file exists, sending completion message"
            outcome_json='"completed with outcome"'
        else
            outcome_json='"completed"'
        fi
        report_status "$deployment_id" "completed" "$container_name" "" "$outcome_json" || true
    else
        local error_msg="agent exited with code $exit_code"
        log "ERROR: Deployment $deployment_id failed: $error_msg"
        report_status "$deployment_id" "failed" "$container_name" "$error_msg" || true
    fi

    return $exit_code
}

# Main loop: poll for pending deployments and execute them
main() {
    log "Supervisor starting: id=$SUPERVISOR_ID conductor=$CONDUCTOR_URL"
    log "Binary: $CRABJAR_BINARY, Workspace root: $WORKSPACE_ROOT"

    # Verify prerequisites
    if ! command -v podman &>/dev/null; then
        log "ERROR: podman not found in PATH"
        exit 1
    fi
    if ! command -v jq &>/dev/null; then
        log "ERROR: jq not found in PATH"
        exit 1
    fi
    mkdir -p "$WORKSPACE_ROOT"

    # Register heartbeat with conductor
    log "Sending initial heartbeat..."
    curl -s -X POST "${CONDUCTOR_URL}/workers/heartbeat/supervisor" \
        -H "Content-Type: application/json" \
        -d "{\"id\":\"$SUPERVISOR_ID\",\"ready\":true,\"health\":{\"active_deployments\":0}}" || true

    while true; do
        # Check concurrent deployments
        local active_count
        active_count=$(podman ps --filter "name=cj-deploy-" --format "{{.Names}}" 2>/dev/null | wc -l)

        if [ "$active_count" -ge "$MAX_CONCURRENT" ]; then
            log "At max concurrent deployments ($active_count/$MAX_CONCURRENT), skipping poll"
            sleep "$POLL_INTERVAL"
            continue
        fi

        # Poll for pending deployments
        local response
        response=$(curl -s "${CONDUCTOR_URL}/workers/deployments/pending/${SUPERVISOR_ID}" 2>&1) || {
            log "WARN: Failed to poll conductor: $response"
            sleep "$POLL_INTERVAL"
            continue
        }

        local pending_count
        pending_count=$(echo "$response" | jq -r '.pending | length' 2>/dev/null || echo "0")

        if [ "$pending_count" -eq 0 ]; then
            # Send heartbeat periodically
            curl -s -X POST "${CONDUCTOR_URL}/workers/heartbeat/supervisor" \
                -H "Content-Type: application/json" \
                -d "{\"id\":\"$SUPERVISOR_ID\",\"ready\":true,\"health\":{\"active_deployments\":$active_count}}" || true
            sleep "$POLL_INTERVAL"
            continue
        fi

        log "Found $pending_count pending deployment(s)"

        # Process each pending deployment (one at a time per poll cycle)
        local first_deployment
        first_deployment=$(echo "$response" | jq -c '.pending[0]')
        local deployment_id
        deployment_id=$(echo "$first_deployment" | jq -r '.id')
        local task_spec_json
        task_spec_json=$(echo "$first_deployment" | jq -c '.task_spec')

        # Run in background to allow polling to continue
        (
            run_deployment "$deployment_id" "$task_spec_json" || true
        ) &
        log "Spawned background deployment for $deployment_id (pid $!)"

        # Brief pause before next poll to avoid hammering
        sleep 2
    done
}

main "$@"
