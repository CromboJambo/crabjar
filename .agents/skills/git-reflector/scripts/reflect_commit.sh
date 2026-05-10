#!/bin/bash

# git-reflector: Automates the Git commit process for decision blobs.
# Usage: ./reflect_commit.sh <path_to_decision_json> [commit_message]

set -e

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <path_to_decision_json> [commit_message]"
    exit 1
fi

DECISION_FILE=$1
COMMIT_MSG=${2:-"Automated decision commit via git-reflector"}

if [ ! -f "$DECISION_FILE" ]; then
    echo "Error: File $DECISION_FILE not found."
    exit 1
fi

# Ensure we are in a git repository or can find it
if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    echo "Error: Not in a git repository. Please run this from the project root."
    exit 1
fi

# Add the file to the index
git add "$DECISION_FILE"

# Commit the change
# We use --no-edit to keep the process automated, or pass a custom message
if [ -n "$2" ]; then
    git commit -m "$COMMIT_MSG"
else
    git commit -m "$COMMIT_MSG"
fi

echo "Successfully committed $DECISION_FILE with message: $COMMIT_MSG"
