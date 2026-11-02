---
name: reddit-reading
description: |
  Use when reading Reddit content including subreddits, searching for topics, viewing threads, and examining user profiles without a browser. Trigger this when the user says "check Reddit" or needs to read Reddit content programmatically.
category: research
tags: [reddit, social-media, reading]
---

# Reddit Reading

This skill provides workflows for reading Reddit content including subreddits, searching for topics, viewing threads, and examining user profiles without requiring a browser — using API access or web scraping as appropriate.

## Core Workflow

1. **Identify target**: Determine subreddit, search query, or thread URL
2. **Access content**: Retrieve relevant Reddit content via appropriate method
3. **Parse and summarize**: Extract key information from posts and comments
4. **Present findings**: Provide organized summary of relevant content
5. **Follow up** (optional): Dive deeper into specific threads or topics

## Usage Instructions

### Input Requirements
* Subreddit name, search query, or thread URL
* Topic of interest
* Time range (optional)

### Example Prompts
* `/reddit-reading check r/rust for recent discussions about async`
* `Search Reddit for opinions on this topic`
* "Read this Reddit thread and summarize the key points"

## Reference Files
* `references/reddit-api-guide.md`: Guide to accessing Reddit content programmatically
* `references/search-strategies.md`: Effective search strategies for finding relevant Reddit content

## Bundled Scripts
None — this skill provides reading methodology and guidance.