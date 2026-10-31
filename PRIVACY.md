# Privacy Policy

**Last updated:** October 31, 2026

## Overview

This Privacy Policy describes how the Crabjar Discord bot fleet collects, uses, stores, and protects your information when you interact with our agents on Discord.

## Information We Collect

### Direct Interaction Data
- **Messages**: Text you send to bots in Discord channels or direct messages
- **Commands**: Specific instructions and parameters you provide
- **User identification**: Discord user ID for routing and permission checks

### System Operation Data
- **Command execution logs**: Records of actions taken by bots (for debugging)
- **Performance metrics**: Response times, token usage, model selection
- **Error reports**: Stack traces and context when commands fail

### Infrastructure Data
- **Machine status**: CPU, memory, disk, network from managed systems
- **Service health**: Uptime and availability of monitored services
- **LLM inference stats**: Model loading, eviction, and throughput data

## How We Use Information

- Execute requested commands and automation workflows
- Route messages to appropriate specialized bots
- Debug issues and improve system reliability
- Monitor fleet health and resource utilization
- Generate status reports and alerts

## Data Storage

All data is stored on private home-lab infrastructure:
- **Primary storage**: Local SSD arrays with RAID redundancy
- **Database**: SQLite instances per service (state management)
- **Logs**: Rotating log files with 30-day retention
- **Backups**: Automated daily snapshots to secondary storage

## Data Sharing

We do not sell or share your data. Information may be disclosed to:
- Other fleet bots for coordinated operations
- System administrators for troubleshooting
- Discord API (required for message delivery)

## Retention Periods

| Data Type | Retention |
|-----------|-----------|
| Command logs | 30 days |
| Error reports | 90 days |
| Performance metrics | 7 days |
| User interaction history | Until manually cleared |

## Security Measures

- All internal communication uses TLS encryption
- Access control via role-based permissions
- Regular security updates to all managed systems
- Isolated network segments for different service tiers

## Your Rights

You may request:
- Export of your interaction data
- Deletion of stored personal information
- Clarification on data processing activities

Contact maintainers through the project repository to exercise these rights.

## Children's Privacy

The Crabjar fleet is not designed for children under 13. We do not knowingly collect information from children.

## Changes to This Policy

We may update this Privacy Policy periodically. Significant changes will be announced through official channels. Continued use after changes constitutes acceptance.

## Contact

For privacy inquiries, contact the Crabjar project maintainers through the official repository.
