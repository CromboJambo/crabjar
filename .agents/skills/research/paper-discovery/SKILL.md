---
name: paper-discovery
description: |
  Use when discovering relevant academic papers on a topic, including searching by keywords, following citation trails, and identifying influential or recent work. Trigger this when the user says "find papers on" or needs academic paper discovery.
category: research
tags: [papers, discovery, academic]
---

# Academic Paper Discovery

This skill provides workflows for discovering relevant academic papers on a topic, including searching by keywords, following citation trails (both forward and backward), and identifying influential or recent work in a field.

## Core Workflow

1. **Identify seed papers**: Find foundational or highly-cited papers on the topic
2. **Search by keywords**: Query databases with relevant terminology
3. **Follow citation trails**: Explore papers that cite seed papers (forward) and papers they cite (backward)
4. **Evaluate relevance**: Assess each paper's applicability to research question
5. **Organize findings**: Create structured list of discovered papers with annotations

## Usage Instructions

### Input Requirements
* Topic or research area
* Keywords (optional)
* Time range (optional)

### Example Prompts
* `/paper-discovery find recent papers on attention mechanisms`
* `Discover influential papers in this research area`
* "What are the key papers I should read on this topic?"

## Reference Files
* `references/discovery-strategies.md`: Strategies for comprehensive paper discovery
* `references/database-comparison.md`: Comparison of academic search databases and their strengths

## Bundled Scripts
None — this skill provides discovery methodology and guidance.