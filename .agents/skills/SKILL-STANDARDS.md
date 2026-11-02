# Skill Documentation Standards

This document defines the SKILL.md format and documentation standards for all crabjar skills.

## SKILL.md Structure

Every skill must have a `SKILL.md` file with the following structure:

```yaml
---
name: skill-name
description: |
  Use when <trigger>. <one-line behavior>.
category: category-name
tags: [tag1, tag2, tag3]
---

# Skill Name

<One-paragraph description of what this skill does and when to use it.>

## Core Workflow

1. Step one
2. Step two
3. Step three

## Usage Instructions

### Input Requirements
* Required input 1
* Required input 2

### Example Prompts
* `/skill-name example command`
* `Use the skill-name skill to...`

## Reference Files
* `references/file.md`: Description of reference file

## Bundled Scripts
* `scripts/script.sh`: Description of script
```

## Field Definitions

### name (required)
- Lowercase, hyphenated identifier
- Max 64 characters
- Must match the directory name

### description (required)
- First line: "Use when <trigger>. <one-line behavior>."
- Keep first 57 chars self-contained as trigger phrase
- Can span multiple lines using YAML block scalar

### category (required)
- One of the standard crabjar categories:
  - `github` — GitHub workflow skills
  - `devops` — Development operations and infrastructure
  - `productivity` — Document creation, presentations, spreadsheets
  - `research` — Academic research, paper discovery
  - `software-development` — Software engineering workflows
  - `autonomous-ai-agents` — Multi-agent orchestration
  - `mlops` — Machine learning operations
  - `finance` — Financial tools and workflows
  - `social-media` — Social platform interactions
  - `note-taking` — Knowledge management and organization
  - `terminal-browser` — Browser automation skills
  - `video` — Video processing and analysis
  - `image-gen` — Image generation tools
  - `computer-use` — Desktop interaction tools

### tags (optional)
- Array of lowercase, hyphenated identifiers
- Used for search and discovery
- Examples: `testing`, `debugging`, `deployment`, `review`

## Body Sections

### Core Workflow
- Numbered steps describing the skill's execution flow
- Should be clear enough to follow without reading reference files

### Usage Instructions
- Input requirements (what the user needs to provide)
- Example prompts showing how to invoke the skill

### Reference Files (optional)
- List of supporting documentation files
- Path relative to skill directory
- Brief description of each file's purpose

### Bundled Scripts (optional)
- List of executable scripts included with the skill
- Path relative to skill directory
- Brief description of each script's purpose

## Quality Standards

1. **Imperative voice**: Write steps as commands ("Run this", "Check that")
2. **Concrete examples**: Include real command examples, not placeholders
3. **Error handling**: Mention common failure modes and how to handle them
4. **Dependencies**: List any required tools or packages
5. **Testing**: Skills should be testable against the automated framework

## Testing Framework Integration

Skills are validated against an automated testing framework that checks:
- SKILL.md format compliance (YAML frontmatter, required fields)
- Category and tag validation
- Reference file existence
- Script executability
- Basic functionality tests (where applicable)

Run validation with: `just validate-skills`