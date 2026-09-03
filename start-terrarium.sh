#!/bin/bash
# Run terrarium detached from terminal

cd ~/projects/crabjar

# Launch terrarium in background, detaching from TTY
nohup bash -c 'exec env COLUMNS=80 LINES=40 ./target/debug/crabjar-terrarium </dev/null >/tmp/terrarium.log 2>&1' &

echo "Terrarium started (check /tmp/terrarium.log)"