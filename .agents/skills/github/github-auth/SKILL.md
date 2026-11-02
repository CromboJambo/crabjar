---
name: github-auth
description: |
  Use when setting up GitHub authentication for crabjar, including HTTPS tokens, SSH keys, and gh CLI login. Trigger this when the user says "set up GitHub auth", "configure gh", or encounters authentication errors with GitHub operations.
category: github
tags: [authentication, setup, configuration]
---

# GitHub Auth Setup

This skill guides users through setting up GitHub authentication for crabjar operations. Supports HTTPS tokens, SSH keys, and the gh CLI.

## Core Workflow

1. **Determine auth method**: Ask user preference (HTTPS token vs SSH key)
2. **Generate credentials** (if needed): Create new token or SSH key pair
3. **Configure git**: Set up `git config` with appropriate credentials
4. **Verify authentication**: Test connection to GitHub
5. **Configure gh CLI** (optional): Install and login if user wants gh CLI

## Usage Instructions

### Input Requirements
* User's GitHub username or organization
* Preferred auth method (HTTPS token or SSH key)
* Repository URL (for verification)

### Example Prompts
* `/github-auth set up HTTPS token authentication for github.com`
* `Configure GitHub SSH authentication using existing key at ~/.ssh/id_ed25519`
* "I'm getting auth errors when pushing to GitHub — help me fix it"

## Reference Files
* `references/gh-cli-setup.md`: Detailed gh CLI installation and configuration
* `references/token-scopes.md`: Recommended token scopes for different use cases

## Bundled Scripts
* `scripts/verify-auth.sh`: Verify current GitHub authentication status