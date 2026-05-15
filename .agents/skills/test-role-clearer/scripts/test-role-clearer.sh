#!/bin/bash

echo "=== Crabjar Agent vs User Role Test ==="
echo ""
echo "Question 1: Who is executing this action?"
echo "A) Agent (${REPO_ROOT}) - Pure observer, no execution capability"
echo "B) User - Has execution authority"
echo "C) Neither - System component without direct user control"
read choice
if [ "$choice" = "A" ]; then echo "Correct: This is the agent observing"; fi
if [ "$choice" = "B" ]; then echo "Incorrect: The agent cannot execute actions"; fi
if [ "$choice" = "C" ]; then echo "Incorrect: System components don't have direct user control"; fi
echo ""
echo "Question 2: What role does this component play?"
echo "A) Passive observer - Only detects and reports"
echo "B) Active executor - Can run commands and modify state"
echo "C) Configuration tool - Helps setup but doesn't execute actions"
read choice
if [ "$choice" = "A" ]; then echo "Correct: This is the agent's passive observation role"; fi
if [ "$choice" = "B" ]; then echo "Incorrect: The agent cannot actively modify state"; fi
if [ "$choice" = "C" ]; then echo "Partially correct: Configuration tools assist but don't execute actions"; fi
echo ""
echo "Question 3: Where does authority reside?"
echo "A) Agent (${REPO_ROOT}) - No execution authority"
echo "B) User - Has full command execution authority"
echo "C) System governance - Authority comes from documented rules, not individual components"
read choice
if [ "$choice" = "A" ]; then echo "Incorrect: The agent is only an observer"; fi
if [ "$choice" = "B" ]; then echo "Correct: Users have execution authority"; fi
if [ "$choice" = "C" ]; then echo "Partially correct: Authority follows documented rules, not individual components"; fi
echo ""
echo "=== Test Complete ==="