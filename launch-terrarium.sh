#!/bin/bash
# Launch terrarium in herdr pane without blocking

cd ~/projects/crabjar

# Send command to specific pane (replace w18 with your actual workspace number)
PANE="${1:-w18:p1}"

echo "Launching terrarium in $PANE..."

# Use a subshell that reads from /dev/null and writes to stdout
# TERR_MODE=text forces text rendering (no ratatui/pty required)
herdr pane send-text "$PANE" "bash -c 'exec env COLUMNS=40 LINES=20 TERR_MODE=text ./target/debug/crabjar-terrarium </dev/null'"

echo "Check your herdr workspace $PANE to see the terrarium running!"
