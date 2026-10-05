#!/bin/bash
# CrabJar agent task loop - uses current crabjar CLI interface
set -e

AGENT_HOME="/home/crombo/.crabjar-agent"
INBOX="$AGENT_HOME/inbox"
OUTBOX="$AGENT_HOME/outbox"
LOG_DIR="$AGENT_HOME/log"
WORKSPACE="$AGENT_HOME/workspace"
CRABJAR="/usr/local/bin/crabjar"

mkdir -p "$LOG_DIR" "$INBOX" "$OUTBOX" "$WORKSPACE"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') $1" >> "$LOG_DIR/agent.log"
}

process_task_file() {
    local task_file="$1"
    local task_id=$(basename "$task_file" .json | sed 's/task_//')
    
    log "Processing task: $task_id"
    
    cd "$WORKSPACE"
    
    # Read the intent from the task file
    local intent=$(cat "$task_file" | grep -o '"intent":"[^"]*"' | sed 's/"intent":"//; s/"$//' | head -c 2000)
    
    if [ -z "$intent" ]; then
        log "Error: could not parse intent from task file $task_id"
        mv "$task_file" "$LOG_DIR/completed_${task_id}.json"
        return 1
    fi
    
    # Extract repo URL if present in intent
    local repo_url=$(echo "$intent" | grep -o 'https://github.com/[^ ]*' | head -1)
    
    if [ -n "$repo_url" ]; then
        log "Found repo URL: $repo_url"
        
        # Clone or update the repo
        local repo_name=$(basename "$repo_url" .git)
        if [ ! -d "$repo_name" ]; then
            git clone "$repo_url" >> "$LOG_DIR/git.log" 2>&1 || true
        else
            cd "$repo_name" && git pull >> "$LOG_DIR/git.log" 2>&1 || true
        fi
        
        # Create state doc via crabjar CLI
        local analysis=""
        if [ -f "README.md" ]; then
            analysis=$(head -50 README.md)
        fi
        
        if [ -n "$analysis" ]; then
            "$CRABJAR" state annotate "$repo_name" "$analysis" >> "$LOG_DIR/crabjar.log" 2>&1 || true
        fi
        
        log "State doc created for $repo_url"
    else
        log "No repo URL found in intent: $intent"
    fi
    
    # Move task to completed
    mv "$task_file" "$LOG_DIR/completed_${task_id}.json"
    
    return 0
}

while true; do
    # Check for tasks in inbox
    TASK_FILE=$(ls -t "$INBOX"/task_*.json 2>/dev/null | head -1)
    
    if [ -z "$TASK_FILE" ]; then
        sleep 5
        continue
    fi
    
    process_task_file "$TASK_FILE" || true
    
    sleep 5
done
