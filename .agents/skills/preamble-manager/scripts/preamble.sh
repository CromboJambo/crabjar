#!/usr/bin/env bash
# load preamble files into opencode instructions
set -euo pipefail

CONFIG_DIR="${CONFIG_DIR:-$HOME/.config/opencode}"
AGENTS_FILE="$CONFIG_DIR/AGENTS.md"

action="${1:-load}"

case "$action" in
    load)
        # Read configured preamble paths from opencode.json
        if [[ -f "$CONFIG_DIR/opencode.json" ]]; then
            paths="$(jq -r '.instruction_paths[]' "$CONFIG_DIR/opencode.json" 2>/dev/null || echo 'none')"
            for path in $paths; do
                if [[ -f "$path" ]]; then
                    echo "--- preamble: $path ---"
                    cat "$path"
                else
                    echo "--- missing: $path ---"
                fi
            done
        else
            echo '{"error": "opencode.json not found"}'
            exit 1
        fi
        ;;
    config)
        # Write preamble paths to opencode.json
        new_paths="${2:-}"
        if [[ -f "$CONFIG_DIR/opencode.json" ]]; then
            jq ".instruction_paths += [$new_paths]" "$CONFIG_DIR/opencode.json" > "$CONFIG_DIR/opencode.json.tmp"
            mv "$CONFIG_DIR/opencode.json.tmp" "$CONFIG_DIR/opencode.json"
            echo '{"status": "updated", "paths": "'"$new_paths"'"}'
        else
            echo '{"error": "opencode.json not found"}'
            exit 1
        fi
        ;;
    symlink)
        # Create symlink from dotfile to opencode instructions
        dotfile="${2:-}"
        if [[ ! -f "$dotfile" ]]; then
            echo '{"error": "dotfile not found", "path": "'"$dotfile"'"}'
            exit 1
        fi
        symlink="$CONFIG_DIR/AGENTS.md"
        ln -sf "$dotfile" "$symlink"
        echo '{"status": "symlinked", "source": "'"$dotfile"'"}'
        ;;
    *)
    echo '{"error": "unknown action", "usage": "preamble.sh [load|config|symlink]"}'
    exit 1
    ;;
esac
