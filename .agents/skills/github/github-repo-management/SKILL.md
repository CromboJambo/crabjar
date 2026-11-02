---
name: github-repo-management
description: |
  Use when cloning, creating, forking repositories, managing remotes, and handling releases via gh CLI or REST API. Trigger this when the user says "clone repo", "create repository", or needs repository management operations.
category: github
tags: [repositories, management, releases]
---

# GitHub Repository Management

This skill provides comprehensive repository management workflows including cloning, creation, forking, remote management, and release handling.

## Core Workflow

1. **Clone repositories**: Clone with appropriate depth and branch settings
2. **Create repositories**: Initialize new repos with templates and configuration
3. **Fork repositories**: Create forks for contribution workflows
4. **Manage remotes**: Configure upstream/origin relationships
5. **Handle releases**: Create, update, and manage versioned releases

## Usage Instructions

### Input Requirements
* Repository URL or owner/repo identifier
* Operation type (clone, create, fork, release)
* Configuration options (branch, depth, template)

### Example Prompts
* `/github-repo-management clone https://github.com/owner/repo --depth 1`
* `Create a new private repository with README and .gitignore`
* "Fork this repository and set up upstream tracking"

## Reference Files
* `references/release-process.md`: Recommended release workflow
* `references/repository-templates.md`: Templates for different repo types

## Bundled Scripts
* `scripts/setup-upstream.sh`: Configure upstream/origin remote relationships