#!/bin/bash
# Run terrarium in detached mode for herdr

cd ~/projects/crabjar

# Create a temp directory for logs
mkdir -p /tmp/herdr-terrarium

# Launch with stdin/stdout redirected to avoid blocking
exec env COLUMNS=80 LINES=40 ./target/debug/crabjar-terrarium \
    </dev/null \
    >/tmp/herdr-terrarium/terr_$(date +%s).log \
    2>&1 &

PID=$!
echo "Terrarium started with PID $PID"