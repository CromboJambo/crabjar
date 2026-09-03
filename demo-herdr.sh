#!/bin/bash
# Crabjar Demo - Launches terrarium and glider in separate herdr panes

cd ~/projects/crabjar

# Create workspace
herdr workspace create --cwd . --label "crabjar-demo" --no-focus

# Get pane IDs from output
PANE1=$(herdr workspace list | grep -o '"pane_id":"[^"]*"' | tail -1 | cut -d'"' -f4)
PANE2=$(herdr pane split $PANE1 --direction down 2>&1 | grep -o '"pane_id":"[^"]*"' | tail -1 | cut -d'"' -f4)

echo "Created workspace with panes: $PANE1 and $PANE2"

# Launch terrarium in top pane (with timeout so it doesn't block forever)
herdr pane send-text $PANE1 "nohup ./target/debug/crabjar-terrarium > /tmp/terrarium.log 2>&1 &"

# Wait a moment for process to start
sleep 1

# Launch glider in bottom pane (with timeout)
herdr pane send-text $PANE2 "nohup ./target/debug/crabjar-glider -m bench -g gospergun > /tmp/glider.log 2>&1 &"

echo "Launched terrarium in $PANE1 and glider in $PANE2"
echo ""
echo "To view outputs:"
echo "  herdr pane read $PANE1 --source visible --lines 50"
echo "  herdr pane read $PANE2 --source visible --lines 50"
