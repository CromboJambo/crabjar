---
id: EXPL-003
title: CrabJar Exhaust as Game Engine Inputs
status: Exploring
date: 2026-10-01
see_also: [[ADR-007]], [[EXPL-001]]
---

# EXPL-003: CrabJar Exhaust as Game Engine Inputs

## Status

Exploring. This examines the idea of treating CrabJar's structured output (exhaust) as a stream of deterministic inputs to a game engine — bridging the gap between agent decision-making and executable action sequences in simulated environments.

## The Core Idea

CrabJar already produces structured JSON exhaust describing actions taken:

```json
{
  "action": "move_file",
  "source": "/tmp/q3.csv",
  "dest": "/archive/2026-10/q3.csv",
  "reason": "stale_export_cleanup"
}
```

What if this exhaust could be fed directly into a game engine as controller inputs? The same way a speedrunner's button presses drive a character through levels, CrabJar's exhaust could drive an entity through a simulated environment.

```pseudocode
// CrabJar decides on action
let decision = crabjar_decide(observation);

// Translate to game input
let input = translate_to_game_input(decision);
// { frame: 142, button: UP, duration: 3 }

// Inject into emulator/game engine
emulator.inject_input(input);
```

## Why This Matters

### 1. Replayability and Determinism
Game emulators are deterministic given the same input sequence. If CrabJar's exhaust is well-structured, we can:
- Replay any agent session exactly
- Compare outcomes across different decision paths
- Debug by rewinding to specific decision points

```pseudocode
// Record a session
let session = crabjar_session({ record: true });
session.execute(task);

// Replay it later (same inputs → same outputs)
emulator.load_recording(session.exhaust);
emulator.playback();
```

### 2. The Speedrunner Analogy Made Concrete
The ChatGPT conversation identified a powerful parallel: speedrunners use savestates and rewind to optimize their input sequences. This is exactly what earned autonomy (ADR-007) does — but in the real world instead of a game emulator.

```pseudocode
// Traditional speedrunning loop
while (!reached_goal()) {
    let checkpoint = emulator.save_state();
    let inputs = generate_attempt();
    emulator.playback(inputs);
    
    if (emulator.reached_goal()) break;
    
    // Analyze failure, adjust strategy
    let lesson = analyze_failure(emulator.state_history());
    update_strategy(lesson);
}

// CrabJar earned autonomy loop (real-world equivalent)
while (!task_complete()) {
    let decision = crabjar_decide(observation);
    let result = execute_decision(decision);
    
    if (result.success) break;
    
    // Learn from failure, update trust model
    update_trust(decision, result);
}
```

The key insight: **the emulator is just a testbed for the earned autonomy loop**. In production, there's no rewind — but the same decision-making structure applies.

### 3. Emergent Behavior Testing
Feed CrabJar's exhaust into different game environments to see what kinds of behaviors emerge:

- Can it navigate a maze? (pathfinding)
- Can it solve a puzzle? (planning)
- Can it optimize resource use? (efficiency)

This becomes a way to **test agent capabilities in controlled, measurable environments** before deploying to real tasks.

## Architecture Sketch

```pseudocode
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│  CrabJar    │────▶│ Input        │────▶│ Game Engine  │
│  Decision   │     │ Translator   │     │ (emulator)   │
│  Engine     │◀────│              │◀────│              │
└─────────────┘     └──────────────┘     └──────────────┘
        ▲                    ▲                    │
        │                    │                    ▼
        └────────────────────┴────────────────────┘
                     Observation
```

### Components

1. **CrabJar Decision Engine**: Existing architecture (guard, concierge, Jev decision layer)
2. **Input Translator**: Maps CrabJar's action schema to game-specific inputs
3. **Game Engine/Emulator**: Any deterministic environment that accepts controller input
4. **Observation Pipeline**: Captures game state and feeds back to CrabJar

### Input Translator Example (Hypothetical)

```pseudocode
// crabjar_action → SMB controller input
fn translate_to_smb(action: CrabAction): SMBInput {
    switch (action.type) {
        case "move_forward":
            return { button: RIGHT, duration: action.duration };
        case "jump":
            return { button: A, duration: 1 };
        case "wait":
            return { button: NONE, duration: action.duration };
        // ...
    }
}
```

## Relationship to Earned Autonomy (ADR-007)

This integration provides a natural testing ground for earned autonomy:

1. **Start at PROPOSE level**: CrabJar suggests actions, human approves/rejects via game playthrough
2. **Track approval history**: Record which action classes succeed in which game states
3. **Earn ACT level**: After N successful similar actions, CrabJar can act autonomously
4. **Topological uncertainty**: Novel game situations (sparse lattice regions) trigger more cautious behavior

```pseudocode
// Example: Learning to navigate a maze
let action_class = "move_toward_exit";

for (attempt in attempts) {
    let decision = crabjar_decide(observation);
    
    if (action_class.permission == PROPOSE) {
        // Show human what CrabJar wants to do
        let approved = human_approves(decision);
        if (!approved) continue;
    }
    
    // Execute in game
    let result = execute_in_game(decision);
    
    // Update earned autonomy
    update_action_class(action_class, decision.action, result.success);
}
```

## Potential Game Engine Integrations

### 1. Retro Gaming Emulators (NES, SNES, etc.)
- Well-understood input schemas
- Deterministic execution
- Rich ecosystem of ROMs for testing
- Perfect for speedrunner-style optimization

### 2. Modern Open-Source Engines (Godot, Unity)
- More complex action spaces
- Better graphics for visualization
- Can test more sophisticated agent behaviors

### 3. Custom Minimal Environments
- Build simple grid-world or physics environments specifically for testing
- Full control over observation and action schemas
- Useful for academic-style RL experiments

## Connection to Sparse Lattice (EXPL-001)

The game environment provides a natural substrate for the sparse neural lattice:

- **Nodes**: Game states (positions, inventory, health, etc.)
- **Edges**: Actions that transition between states
- **Edge weights**: Learned value of taking action in given state
- **Sparsity**: Only explore/learn about reachable states, not all possible states

```pseudocode
// Build lattice from game experience
let node = create_state_node(game_state);
let edge = create_action_edge(node, decision.action, result.reward);
lattice.add_edge(edge);

// Query lattice for best action in similar state
let similar_nodes = lattice.find_similar(current_state);
let best_action = lattice.query_best_action(similar_nodes);
```

## Open Questions

1. **Determinism**: How do we ensure the game engine runs deterministically? (Seed RNG, fixed timestep, etc.)
2. **Observation fidelity**: What does CrabJar "see"? Full state access or limited sensors?
3. **Action granularity**: How fine-grained should actions be? (Raw button presses vs. high-level commands)
4. **Performance**: Can CrabJar make decisions fast enough for real-time gameplay? Or do we need to operate at simulation speed?

## Next Steps for Exploration

1. Pick a simple game environment (e.g., Snake, Pong, or a grid-world maze)
2. Build minimal input translator from CrabJar actions to game inputs
3. Implement basic observation pipeline (game state → CrabJar input)
4. Test earned autonomy loop: Can CrabJar learn to play through repeated attempts?
5. Measure: Does the sparse lattice approach yield better policies than dense models for these tasks?
