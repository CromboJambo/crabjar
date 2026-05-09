---
name: action-policy
description: |
  Implement action policy gating for destructive action authorization. Use whenever the user wants to gate destructive actions via JSON file — action policy prevents unauthorized destructive actions like form submission, deletion, navigation to sensitive pages. Trigger when the user mentions action policy, action gating, destructive actions, authorization layer, or wants to build an execution gate for web actions.
---

# Action Policy

## Core Concept

Action policy gates destructive actions via JSON file. This separates detection from authorization — knowing an element exists does not grant the right to act on it. Actions are classified by risk level and gated by policy.

## Action Classification

| Category | Risk | Examples |
|---|---|---|
| **Safe** | Low | click, get text, get html, screenshot |
| **Sensitive** | Medium | fill, type, submit, navigate |
| **Destructive** | High | delete, logout, clear storage, dismiss dialog |
| **Critical** | Extreme | auth changes, credential manipulation, domain switch |

## Policy Format

```json
{
  "categories": {
    "safe": {
      "allowed": true,
      "confirmation": false
    },
    "sensitive": {
      "allowed": true,
      "confirmation": true
    },
    "destructive": {
      "allowed": false,
      "confirmation": true
    },
    "critical": {
      "allowed": false,
      "confirmation": true
    }
  },
  "domain_restrictions": {
    "example.com": {
      "sensitive": {
        "allowed": true
      }
    }
  },
  "element_restrictions": {
    "@e1": {
      "click": {
        "allowed": false
      }
    }
  }
}
```

## Implementation

### 1. Action Classification

Classify each action by risk category:
- **Safe**: observation-only actions
- **Sensitive**: user-facing actions
- **Destructive**: irreversible actions
- **Critical**: auth/security actions

### 2. Policy Check

Check action against policy:
- **Category gate**: allowed/confirmation required
- **Domain gate**: domain-specific restrictions
- **Element gate**: specific element restrictions
- **Context gate**: session/state context restrictions

### 3. Result Handling

On policy check result:
- **Allowed**: proceed with action
- **Confirmation required**: surface confirmation request
- **Blocked**: return `Interrupted` with reason
- **Pending**: return `Pending` to queue for review

### 4. Uncertainty Surface

If confidence is below threshold:
- Surface uncertainty before executing
- Log uncertainty in decision record
- Require additional confirmation

## Configuration

- **Policy file**: JSON file with action classifications
- **Policy source**: CLI flags, config file, env vars
- **Dynamic updates**: policy can be updated mid-session
- **Confirmation mode**: manual, automated, required
- **Threshold**: confidence threshold for uncertainty surface

## Integration with Crabjar

Crabjar's authorization layer should adopt:
- Action policy as gate for destructive actions
- Category classification for risk assessment
- Domain-specific restrictions
- Element-specific restrictions
- Uncertainty surface before execution
- Pending queue for review actions

## References

Read `state-docs/agent-browser-state.md` section 2.6 for security features context and section 7.5 for stale detection thresholds.
