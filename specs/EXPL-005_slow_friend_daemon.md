---
id: EXPL-005
title: The Slow Friend Daemon Architecture
status: Exploring
date: 2026-10-01
see_also: [[ADR-007]], [[EXPL-001]]
---

# EXPL-005: The Slow Friend Daemon Architecture

## Status

Exploring. This examines the "slow friend" architecture from the ChatGPT conversation — an autonomous agent that sleeps most of the time and only wakes when something noteworthy happens, with escalating tiers of computational cost.

## The Core Insight

From the conversation: a good autonomous system shouldn't be continuously running inference and consuming tokens. Instead:

> **A daemon that knows when an `if` statement isn't enough.**

The agent's job isn't to occupy computation — it's to make useful decisions *when computation is warranted*. This is the "slow friend" metaphor: a friend who has their own life, notices something relevant, says "hey, this changed," and then goes back to what they were doing.

## Tiered Attention Architecture

```pseudocode
LOW COST
         │
  deterministic
     sensors
         │
   something odd?
    /          \
  no            yes
  │              │
sleep        structural parse
                  │
            meaningful?
             /       \
           no         yes
           │           │
         sleep       Jev decision
                       │
                 worth acting?
                  /          \
                no            yes
                │              │
              sleep       CrabJar proposal
```

Each tier is more expensive computationally but only triggered when cheaper checks detect something interesting. This makes attention itself a scarce resource — the agent doesn't waste it on noise.

## Component Breakdown

### 1. Deterministic Sensors (Cheapest)
Simple rule-based checks that run constantly at low cost:

```pseudocode
// Runs every minute, trivial CPU
fn check_sensors() {
    if (disk_usage() > 90%) return ALERT;
    if (new_emails() > 0) return ALERT;
    if (process_restarted()) return ALERT;
    return QUIET;
}
```

### 2. Structural Interpretation (Medium Cost)
When sensors detect something, parse it for meaning:

```pseudocode
fn interpret_event(event) {
    // Is this email from my boss? About a deadline?
    // Is this disk usage from logs or actual data?
    // Did the process crash or restart cleanly?
    
    let context = gather_context(event);
    return { severity: assess(context), novelty: is_new(context) };
}
```

### 3. Jev Decision Layer (Expensive)
Only invoked for genuinely ambiguous situations that require semantic judgment:

```pseudocode
fn jev_decision(interpretation) {
    // This is where the LLM comes in
    // "Should I escalate this to the user?"
    // "Is this pattern worth learning from?"
    
    return propose_action_or_sleep();
}
```

### 4. CrabJar Execution (Most Expensive / Irreversible)
The actual action with checkpointing and notification:

```pseudocode
fn execute_with_guard(action) {
    let checkpoint = create_checkpoint();
    let result = action.execute();
    
    if (result.failure) {
        rollback(checkpoint);
    } else {
        notify_user("Did X because Y");
    }
}
```

## Implementation Sketch

```pseudocode
// slow_friend.hml — main daemon loop

let last_event_time = now();

while (true) {
    // Tier 1: Cheap sensor checks
    let sensor_result = check_all_sensors();
    
    if (sensor_result == QUIET) {
        sleep(60);  // Nothing interesting, wait
        continue;
    }
    
    // Tier 2: Structural interpretation
    let interpretation = interpret_event(sensor_result);
    
    if (!interpretation.meaningful) {
        sleep(300);  // Noise, ignore for a bit longer
        continue;
    }
    
    // Tier 3: Jev decision (expensive!)
    let decision = jev_decision(interpretation);
    
    if (decision.action == SLEEP) {
        sleep(60);
        continue;
    }
    
    // Tier 4: Execute with CrabJar guard
    execute_with_guard(decision.action);
    
    // Update earned autonomy based on outcome
    update_trust(decision, outcome);
}
```

## Connection to Earned Autonomy (ADR-007)

The slow friend architecture naturally integrates with earned autonomy:

1. **New event types** start at the ASK level — the agent notices something but doesn't know how to handle it
2. **Repeated similar events** build up approval history in the action class registry
3. **Eventually** the agent can handle certain event types autonomously (skip Jev decision, go straight to CrabJar execution)

```pseudocode
// Example evolution
// Week 1: Disk full → Jev decides → proposes cleanup to user
// Week 2-4: User approves disk cleanup 5 times
// Week 5: Disk full → agent cleans up automatically (earned ACT level)
```

## Connection to Sparse Lattice (EXPL-001)

The lattice provides the memory substrate for this architecture:

- **Sensors** detect nodes/edges that match known patterns
- **Interpretation** queries the lattice for similar past events
- **Jev decision** is only needed when the lattice has no answer (sparse region)
- **Learning** adds new edges to the lattice after each decision

```pseudocode
// Lattice query during interpretation
let similar_events = lattice.find_similar(current_event);

if (similar_events.dense) {
    // Well-known pattern, use learned response
    return lattice.query_response(similar_events);
} else {
    // Novel situation, need Jev decision
    return escalate_to_jev();
}
```

## Why This Fits CrabJar's Philosophy

1. **Rust for the cheap stuff**: Deterministic sensors and structural parsing can be written in Rust — fast, efficient, no LLM overhead
2. **Jev for ambiguity**: Only invoke expensive semantic reasoning when truly needed
3. **CrabJar for authorization**: Every action goes through the guard/concierge pipeline regardless of how it was decided
4. **Earned autonomy over time**: The system gets better and more autonomous as it learns

This is fundamentally different from "AI employee" models where an agent is constantly generating text and taking actions. The slow friend is quiet, efficient, and only speaks up when it has something meaningful to say.

## Open Questions

1. **Sensor design**: What events should CrabJar monitor? File changes? Email? System metrics? User-defined triggers?
2. **Interpretation fidelity**: How much context does the structural parser need to correctly classify events?
3. **Jev invocation cost**: Can we batch multiple events into a single Jev decision to reduce token consumption?
4. **User notification thresholds**: When should the agent bother the user vs. handling things silently?

## Next Steps for Exploration

1. Define a minimal set of sensors for a home-lab environment (disk, process health, email)
2. Build the structural interpretation layer for one event type (e.g., "process crashed")
3. Test the escalation path: sensor → interpret → Jev → CrabJar execution
4. Measure: How often does each tier actually trigger? Where's the right balance?
