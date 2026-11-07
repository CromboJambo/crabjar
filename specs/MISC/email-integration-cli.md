# Crabjar Email CLI Interface

Structured query interface for agent-driven email management with approval workflow.

## Account Management

```bash
crabjar email accounts list
crabjar email accounts add --provider gmail --email user@example.com --name "Work"
crabjar email accounts remove <id>
crabjar email sync --account <email_or_id> [--since <datetime>]
```

## Message Listing & Reading

```bash
crabjar email list --account <email> [--label <name>] [--after <date>] [--before <date>] [--limit <n>]
crabjar email read <id> [--format text|html|json]
crabjar email threads --account <email> [--unread] [--limit <n>]
```

## Search (FTS5)

```bash
crabjar email search "<query>" [--account <email>] [--label <name>] [--limit <n>]
# Examples: "invoice from acme" "re: meeting before 2026-10-01"
```

## Drafting & Sending (Approval Required)

```bash
crabjar email draft --to <addr> --subject <subj> --body <text> [--cc <addr>] [--account <email>]
# Returns draft message ID

crabjar email send-request <draft_id>
# Creates approval request; does NOT send

crabjar email approve <request_id>
# Executes the pending action (send/delete/move)

crabjar email reject <request_id>
```

## Label/Folder Management

```bash
crabjar email labels list --account <email>
crabjar email apply-label <message_id> <label_name>
crabjar email remove-label <message_id> <label_name>
crabjar email move <message_id> <folder_name>
```

## Agent Decision Review

```bash
crabjar email decisions list [--status pending|approved|rejected] [--account <email>]
crabjar email decisions show <id>
crabjar email approvals list [--status pending]
```

## Recurrence Tracking

```bash
crabjar email recurrence list [--active]
crabjar email recurrence next <rule_id>
```

## Output Format

All commands return structured JSON:

```json
{
  "success": true,
  "data": { ... },
  "doubt": {
    "assumptions": [],
    "blind_spots": [],
    "last_validation": "2026-10-23T12:00:00Z",
    "stale_after": "2026-10-24T12:00:00Z"
  }
}
```

## Agent Workflow Example

```bash
# Agent searches for invoices
crabjar email search "invoice" --account work@example.com

# Agent classifies and suggests action
crabjar email decisions create --message-id 123 --action suggest_file \
    --details '{"label":"Invoices","confidence":0.95}'

# User reviews pending approvals
crabjar email approvals list

# User approves or rejects
crabjar email approve <request_id>
```