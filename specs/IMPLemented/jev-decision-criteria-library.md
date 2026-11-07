# JEV Decision Criteria Library Specification

**Status:** Proposed  
**Last Updated:** 2026-10-17  
**Author:** crabjar team (CromboJambo)

## Overview

Design for a curated library of reusable decision criteria files for different operational contexts in the crabjar fleet. Each criteria file defines semantic boundaries for automated decision-making via embedding similarity, enabling consistent triage across service alerts, deployment gates, resource management, and alert classification.

The library follows versioned JSON schema with discovery via the existing CLI infrastructure (`crabjar decide list-criteria`) and validation tools (`crabjar decide validate`).

## Architecture

### Library Layout

```
/opt/crabjar/criteria/              # System-wide criteria (read-only)
├── service-triage.json             # Service health triage (4 options)
├── deployment-gate.json            # Deployment readiness (3 options)
├── alert-classification.json       # Alert severity (3 options)
├── resource-management.json        # Resource scaling decisions (3 options)
└── README.md                       # Library documentation

~/.crabjar/criteria/                # User overrides (optional)
├── service-triage.json             # Custom service triage thresholds
└── custom-*.json                   # Fleet-specific criteria
```

### Versioning Strategy

Each criteria file includes semantic versioning in its metadata:

```hemlock
{
    "version": "1.2.0",
    "updated_at": "2026-10-17T15:00:00Z",
    "changelog": "Adjusted 'critical' threshold wording based on historical precision analysis"
}
```

Version bumps follow semantic versioning rules:
- **Major**: Option IDs changed or removed (breaking change for automation)
- **Minor**: New options added or descriptions significantly revised
- **Patch**: Wording tweaks, typo fixes, no semantic change to boundaries

### Schema Definition

All criteria files conform to the JEV Decision Criteria JSON Schema (v2):

```hemlock
{
    "$schema": "https://crabjar.dev/schemas/decision-criteria-v2.json",
    "version": "1.0.0",
    "name": "service-triage",
    "description": "Triage service health alerts into actionable categories",
    "updated_at": "2026-10-17T15:00:00Z",
    
    "options": [
        {
            "id": "restart_service",
            "description": "Service is down or unresponsive; requires immediate restart"
        },
        {
            "id": "disk_full",
            "description": "Disk space exhaustion affecting service operation"
        },
        {
            "id": "network_issue",
            "description": "Network connectivity problem preventing normal operation"
        },
        {
            "id": "ignore",
            "description": "Transient or informational event requiring no action"
        }
    ],
    
    "threshold": 0.75,
    "metadata": {
        "context": "service-health",
        "fleet_scope": "all",
        "tags": ["ops", "triage", "automated"]
    }
}
```

### Loading and Discovery

Criteria loading follows a three-tier priority system:

1. **Explicit path**: `--criteria /path/to/file.json` (highest priority)
2. **System library**: `/opt/crabjar/criteria/<name>.json`
3. **User override**: `~/.crabjar/criteria/<name>.json` (overrides system if present)

Discovery scans both system and user directories, reporting all available libraries with their option counts and versions.

## Criteria Library Contents

### Service Triage (`service-triage.json`)

Four-option triage for service health alerts. The most commonly used criteria in the fleet.

```hemlock
{
    "version": "1.2.0",
    "name": "service-triage",
    "description": "Triage service health alerts into actionable categories",
    "options": [
        {
            "id": "restart_service",
            "description": "Service is down or unresponsive; requires immediate restart"
        },
        {
            "id": "disk_full",
            "description": "Disk space exhaustion affecting service operation"
        },
        {
            "id": "network_issue",
            "description": "Network connectivity problem preventing normal operation"
        },
        {
            "id": "ignore",
            "description": "Transient or informational event requiring no action"
        }
    ],
    "threshold": 0.75,
    "metadata": {
        "context": "service-health",
        "fleet_scope": "all"
    }
}
```

**Example usage:**
```bash
crabjar decide evaluate "homeassistant service not responding on port 8123" \
    --criteria service-triage
# → {"option":"restart_service","confidence":0.847}
```

### Deployment Gate (`deployment-gate.json`)

Three-option gate for deployment readiness decisions. Used before automated deploys to determine if conditions are favorable.

```hemlock
{
    "version": "1.0.3",
    "name": "deployment-gate",
    "description": "Evaluate deployment readiness based on current system state",
    "options": [
        {
            "id": "proceed",
            "description": "System is stable and ready for deployment; no blockers detected"
        },
        {
            "id": "hold",
            "description": "Minor issues present that should be resolved before deploying; not blocking but risky"
        },
        {
            "id": "abort",
            "description": "Critical system instability or active incidents making deployment unsafe"
        }
    ],
    "threshold": 0.80,
    "metadata": {
        "context": "deployment",
        "fleet_scope": "all"
    }
}
```

**Example usage:**
```bash
crabjar decide evaluate "CI passed but production has elevated error rates in last hour" \
    --criteria deployment-gate
# → {"option":"hold","confidence":0.823}
```

### Alert Classification (`alert-classification.json`)

Three-option severity classification for incoming alerts. Routes alerts to appropriate response channels based on urgency.

```hemlock
{
    "version": "1.1.0",
    "name": "alert-classification",
    "description": "Classify alert severity for routing and escalation",
    "options": [
        {
            "id": "critical",
            "description": "Requires immediate human intervention within minutes; service outage or data loss imminent"
        },
        {
            "id": "warning",
            "description": "Degraded performance or approaching thresholds; needs attention within hours"
        },
        {
            "id": "info",
            "description": "Normal operational event for awareness; no action required"
        }
    ],
    "threshold": 0.78,
    "metadata": {
        "context": "alerting",
        "fleet_scope": "all"
    }
}
```

**Example usage:**
```bash
crabjar decide evaluate "Database replication lag exceeded 30 seconds on primary replica" \
    --criteria alert-classification
# → {"option":"warning","confidence":0.812}
```

### Resource Management (`resource-management.json`)

Three-option scaling decision for resource management. Determines whether to scale up, maintain, or scale down based on load patterns.

```hemlock
{
    "version": "1.0.1",
    "name": "resource-management",
    "description": "Evaluate resource utilization for scaling decisions",
    "options": [
        {
            "id": "scale_up",
            "description": "Sustained high resource utilization indicating need for additional capacity"
        },
        {
            "id": "maintain",
            "description": "Resource utilization within normal operating range; no scaling needed"
        },
        {
            "id": "scale_down",
            "description": "Consistently low resource utilization indicating excess capacity"
        }
    ],
    "threshold": 0.72,
    "metadata": {
        "context": "scaling",
        "fleet_scope": "compute-clusters"
    }
}
```

**Example usage:**
```bash
crabjar decide evaluate "CPU utilization averaged 85% over past hour with growing request queue" \
    --criteria resource-management
# → {"option":"scale_up","confidence":0.834}
```

## Validation and Quality Assurance

### Criteria Validation Checks

The `crabjar decide validate` command performs the following checks:

1. **Schema validation**: Required fields present, correct types, valid JSON structure
2. **Option distinctness**: Cosine similarity between all option pairs must be < 0.85 (flags overlapping semantics)
3. **Description quality**: Each description must be a complete sentence with clear semantic boundaries
4. **Threshold sanity**: Threshold must be between 0.5 and 0.95

### Historical Calibration

Criteria thresholds should be calibrated using historical decision data:

```bash
# Analyze past decisions against known outcomes
crabjar decide learn --criteria service-triage --history /var/log/crabjar/decisions.jsonl

# Output includes precision/recall at current threshold and recommended adjustments
{
    "current_threshold": 0.75,
    "precision_at_threshold": 0.82,
    "recall_at_threshold": 0.91,
    "recommended_threshold": 0.78,
    "rationale": "Precision improved by 3% at cost of 2% recall; better tradeoff for automated restarts"
}
```

### Fleet-Wide Consistency

To ensure consistent decision-making across the fleet:

1. System criteria in `/opt/crabjar/criteria/` are version-controlled and deployed atomically
2. User overrides in `~/.crabjar/criteria/` must pass validation before use
3. Criteria changes require `crabjar decide validate` to pass on all affected machines

## Integration with Crabjar Systems

### Guard Layer Integration

Decision criteria can be embedded in guard policies for automated approval/denial:

```hemlock
// In guard policy configuration
{
    "action": "restart_service",
    "risk": "medium",
    "decision_criteria": {
        "criteria_file": "/opt/crabjar/criteria/service-triage.json",
        "require_option": "restart_service",
        "min_confidence": 0.85
    }
}
```

### Alert-Aggregator Integration

Replace regex pattern matching in alert-aggregator with JEV evaluation:

```hemlock
// Old approach (regex)
if (alert.message.contains("ERROR") || alert.message.contains("CRITICAL")) {
    escalate();
}

// New approach (JEV semantic evaluation)
let decision = decide_evaluate(alert.message, "alert-classification");
if (decision.option == "critical" && decision.confidence > 0.85) {
    escalate();
} else if (decision.option == "warning") {
    log_and_monitor();
}
```

### Knowledge Store Confidence Integration

Use semantic similarity scores to calibrate knowledge store confidence levels:

```hemlock
// Replace static confidence defaults with learned values
let similarity = cosine_similarity(observation_embedding, known_issue_embedding);
let confidence = map_similarity_to_confidence(similarity);  // 0.0-1.0
```

## Testing Strategy

### Unit Tests (`tests/criteria/`)

- `schema_validation_passes`: Valid criteria files pass schema checks
- `schema_validation_fails`: Invalid files detected with specific error messages
- `option_distinctness_check`: Overlapping options flagged by similarity threshold
- `threshold_sanity_check`: Out-of-range thresholds rejected

### Integration Tests (`tests/cli/decide_criteria.rs`)

- `list_criteria_discovers_libraries`: All system and user criteria discovered
- `criteria_loading_priority`: User override takes precedence over system default
- `validate_roundtrip`: Generate → validate → evaluate works end-to-end

### Snapshot Tests (`tests/snapshots/criteria/`)

- Each criteria library file has a snapshot test ensuring versioned content stability
- Validation output format for valid and invalid files
- List response with multiple libraries

### Test Fixtures

```
tests/fixtures/criteria/
├── valid-service-triage.json      # Standard 4-option criteria
├── valid-deployment-gate.json     # Standard 3-option criteria  
├── invalid-missing-options.json   # Schema validation test
├── overlapping-criteria.json      # Similarity check test (options too similar)
└── threshold-out-of-range.json    # Sanity check test
```

## Deployment Process

### Adding New Criteria to Fleet

1. Create criteria file in local development environment
2. Validate with `crabjar decide validate <file>`
3. Test with representative observations using hash backend
4. Commit to `/opt/crabjar/criteria/` directory in deployment repo
5. Deploy via standard binary distribution process
6. Update fleet documentation and version changelog

### Updating Existing Criteria

1. Bump version number per semantic versioning rules
2. Document changes in `changelog` field
3. Validate and test as above
4. Deploy updated file to all fleet nodes
5. Monitor decision quality metrics for regression

## Future Work

- **Dynamic criteria generation**: Auto-generate criteria from historical decision patterns using clustering
- **Multi-language support**: Translate criteria descriptions for non-English observations
- **A/B testing framework**: Run old vs new criteria in parallel to measure improvement
- **Criteria composition**: Combine multiple criteria files for complex multi-stage decisions
