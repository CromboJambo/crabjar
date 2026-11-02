---
name: microsoft-365-cli
description: |
  Use when interacting with Microsoft 365 services from the terminal via ICS subscriptions, including calendar management, email operations, and Office app integration. Trigger this when the user says "check my Outlook calendar" or needs M365 CLI operations.
category: productivity
tags: [microsoft-365, office, cli]
---

# Microsoft 365 CLI Integration

This skill provides workflows for interacting with Microsoft 365 services from the terminal via ICS subscriptions, including calendar management, email operations, and Office app integration.

## Core Workflow

1. **Authenticate**: Connect to Microsoft 365 account via OAuth or credentials
2. **Query services**: Retrieve calendar events, emails, contacts as needed
3. **Process data**: Parse and format M365 data for agent consumption
4. **Execute operations**: Create/update/delete items as requested
5. **Report results**: Provide clear summary of operations performed

## Usage Instructions

### Input Requirements
* Microsoft 365 account credentials or OAuth token
* Service to interact with (calendar, mail, contacts)
* Operation type (query, create, update, delete)

### Example Prompts
* `/microsoft-365-cli check my Outlook calendar for today`
* `Send email via Microsoft 365 API`
* "List my upcoming meetings from Office 365"

## Reference Files
* `references/m365-api-guide.md`: Guide to Microsoft 365 REST APIs
* `references/ics-subscription-setup.md`: Setting up ICS subscriptions for calendar access

## Bundled Scripts
None — this skill provides integration methodology and guidance.