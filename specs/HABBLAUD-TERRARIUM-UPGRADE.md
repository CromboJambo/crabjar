# Habblaud-Inspired Terrarium Upgrade

**Status:** Draft  
**Date:** 2026-10-10  
**Inspiration:** [habblaud](https://github.com/marmottajr/habblaud) — animated office visualizing Claude Code agents

## Overview

Upgrade crabjar terrarium from basic task visualization to a rich, Habblaud-style living habitat where agents (crabs) exhibit detailed activity states, idle socialization, and real-time behavioral responses to their work environment.

**Key principle:** Each crab's visible behavior should tell you exactly what it's doing at a glance — working, waiting, blocked, idle-socializing — without reading text labels.

## Architecture Mapping

| Habblaud Component | Crabjar Equivalent | Notes |
|-------------------|-------------------|-------|
| Claude Code sessions | Kanban tasks + guard queue | Source of agent work |
| JSONL transcript parsing | DAGR event stream | Already exists in crabjar |
| Activity classification | Task state → activity mapping | New: map task states to rich activities |
| Room model (project = room) | Task queue/board = zone | Already conceptualized |
| Shell job tracking | Pending action monitoring | Map guard pending actions |
| Permission requests | Blocked tasks needing input | Visual "raise hand" indicator |
| SSE real-time updates | DAGR event emission | Already exists |

## Upgrade Phases

### Phase 1: Rich Activity States (Foundation)

**Goal:** Replace simple task status with Habblaud-style activity classification.

**New CrabActivity enum:**
```rust
pub enum CrabActivity {
    Working,      // Executing a task
    Thinking,     // Planning/analyzing (model generating)
    WaitingShell, // Blocked on external command
    WaitingInput, // Needs human decision (blocked task)
    Idle,         // No tasks assigned
    Socializing,  // Idle + other crabs nearby → animate social behavior
    Done,         // Task completed, waiting for next
    Offline,      // Agent inactive
}
```

**Activity metadata:**
```rust
pub struct ActivityInfo {
    pub kind: CrabActivity,
    pub icon: &'static str,      // "🦀", "💭", "⏳", "🙋", etc.
    pub text: String,             // Short description for UI
    pub detail: Option<String>,   // Longer context (command, task name)
}
```

**Implementation:** Extend `dagr_feed.rs` event types to include activity transitions. Each crab tracks its current activity and emits events when it changes.

### Phase 2: Shell Job Visualization

**Goal:** Show crabs waiting on external commands with progress indicators.

**Features:**
- Crab sits at "terminal" with hourglass animation
- Show command duration (like Habblaud's shell wait timer)
- Visual escalation: short wait → patient; long wait → bored animations
- Command completion celebration (confetti) or failure (rain cloud)

**Data source:** Guard pending actions + orchestrator task execution state.

### Phase 3: Permission/Request Visualization

**Goal:** Crabs "raise hand" when blocked waiting for human input.

**Features:**
- Crab runs to "desk" with raised arm animation
- Pulsing alert indicator (like Habblaud's waiting state)
- Show what's needed: approval, decision, or answer
- Option to approve/answer from terrarium UI (future)

**Data source:** Blocked tasks with `needs_input` reason.

### Phase 4: Idle Socialization

**Goal:** When crabs have nothing to do, they socialize — making the habitat feel alive.

**Social activities (randomized when idle):**
- Watch TV together (multiple idle crabs gather)
- Play ping-pong / video games
- Chat at coffee machine
- Nap individually
- Walk around exploring

**Implementation:** Simple state machine in crab behavior loop:
```rust
if activity == Idle {
    if nearby_idle_crabs > 0 {
        // Social activity with group
    } else {
        // Solo idle activity (nap, explore)
    }
}
```

### Phase 5: Real-Time Event Integration

**Goal:** Wire up DAGR events to drive live habitat updates.

**Event → Behavior mapping:**
- Task assigned → Crab walks to task location
- Task started → Crab begins working animation
- Task blocked → Crab raises hand / waits
- Task completed → Crab celebrates, moves to next
- Shell command running → Crab watches terminal
- Agent idle → Crab socializes or naps

**Implementation:** Extend `command_handler.rs` to process DAGR events and update crab states accordingly.

## Technical Implementation Plan

### File Changes

1. **`src/dagr_feed.rs`** — Add activity event types:
   - `AgentActivityChanged { agent_id, old_activity, new_activity }`
   - `ShellJobStarted { agent_id, command, started_at }`
   - `ShellJobFinished { agent_id, success, duration_ms }`
   - `PermissionRequested { agent_id, request_type, detail }`

2. **`src/render_isometric.rs`** — Add activity-based rendering:
   - Activity-specific crab sprites/poses
   - Animation frames for different activities
   - Social group positioning logic

3. **`src/world.rs`** — Extend Crab struct:
   ```rust
   pub struct Crab {
       // ... existing fields
       pub activity: CrabActivity,
       pub activity_info: Option<ActivityInfo>,
       pub shell_job: Option<ShellJobInfo>,
       pub idle_since: Option<i64>,
   }
   ```

4. **`src/command_handler.rs`** — Process activity events:
   - Map DAGR events to crab state changes
   - Trigger social behavior when appropriate

### Dependencies

- Add animation frame support (extend existing sprite system)
- Add idle timer tracking per crab
- Add proximity detection for social grouping

## Visual Design Principles

From Habblaud's design language, adapted for crabs:

1. **One glance = full status**: Activity should be readable from crab pose/animation alone
2. **Progressive detail**: Hover/click reveals more context (command text, task name)
3. **Emotional states**: Working = focused; waiting = patient/bored; blocked = anxious; idle = relaxed/social
4. **Group dynamics**: Multiple idle crabs should form social clusters

## Success Criteria

- Crab activities are visually distinct without reading labels
- Waiting/blocked states are obvious and show what's needed
- Idle crabs don't look "broken" — they're doing something purposeful
- Real-time updates reflect actual crabjar task state within 1-2 seconds
- Social behaviors emerge naturally from idle time + proximity

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Animation complexity slows rendering | Use simple sprite swaps, not complex animations |
| Social behavior feels random/cheesy | Limit to 3-4 activities; make them subtle |
| Activity mapping incomplete | Start with core states; add edge cases iteratively |
| DAGR event volume overwhelms UI | Throttle updates; batch state changes per tick |

## Next Steps

1. Implement Phase 1 (activity states) — foundation for everything else
2. Wire up real DAGR events from kanban/guard queues
3. Add visual indicators for blocked/waiting states
4. Iterate on idle/social behaviors based on feel
