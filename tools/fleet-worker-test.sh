#!/bin/bash
# Simple fleet worker that polls conductor API for tasks and executes them

CONDUCTOR_URL="http://jambo:8091"
WORKER_NAME="ftw3-script-worker"

echo "Registering as worker: $WORKER_NAME"
REG=$(curl -s -X POST "$CONDUCTOR_URL/workers/register" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"$WORKER_NAME\", \"capabilities\": [\"gpu:nvidia:3070ti\"]}")

TOKEN=$(echo "$REG" | python3 -c "import sys,json; print(json.load(sys.stdin)['auth_token'])")
WORKER_ID=$(echo "$REG" | python3 -c "import sys,json; print(json.load(sys.stdin)['worker_id'])")

echo "Registered with worker_id: $WORKER_ID"
echo "Polling for tasks..."

# Poll for available tasks (simplified - just checks database directly via API)
for i in $(seq 1 3); do
    echo "Poll attempt $i..."
    
    # Try to get pending tasks via the conductor API
    RESPONSE=$(curl -s "$CONDUCTOR_URL/tasks/pending" \
        -H "Authorization: Bearer $TOKEN")
    
    if [ -n "$RESPONSE" ]; then
        echo "Got response: $RESPONSE"
        
        # Parse task ID and command from response
        TASK_ID=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
        COMMAND=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['command'])" 2>/dev/null)
        
        if [ -n "$TASK_ID" ] && [ -n "$COMMAND" ]; then
            echo "Claiming task $TASK_ID: $COMMAND"
            
            # Claim the task
            CLAIM=$(curl -s -X POST "$CONDUCTOR_URL/tasks/$TASK_ID/claim" \
                -H "Content-Type: application/json" \
                -H "Authorization: Bearer $TOKEN" \
                -d "{\"worker_id\": \"$WORKER_ID\"}")
            
            echo "Claim response: $CLAIM"
            
            # Execute the task (simplified)
            echo "Executing: $COMMAND"
            RESULT=$(eval "$COMMAND" 2>&1)
            EXIT_CODE=$?
            
            echo "Task completed with exit code: $EXIT_CODE"
            echo "Output: $RESULT"
            
            # Report result back to conductor
            REPORT=$(curl -s -X POST "$CONDUCTOR_URL/tasks/$TASK_ID/complete" \
                -H "Content-Type: application/json" \
                -H "Authorization: Bearer $TOKEN" \
                -d "{\"worker_id\": \"$WORKER_ID\", \"exit_code\": $EXIT_CODE, \"output\": \"$RESULT\"}")
            
            echo "Report response: $REPORT"
        fi
    else
        echo "No tasks available yet"
    fi
    
    sleep 5
done

echo "Worker shutting down"
