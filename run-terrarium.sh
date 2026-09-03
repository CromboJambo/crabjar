#!/bin/bash
# Wrapper to run terrarium in background without blocking stdin

exec env COLUMNS=80 LINES=40 ./target/debug/crabjar-terrarium < /dev/null 2>&1 &
