---
id: EXPL-004
title: CrabJar Exhaust as Game Controller Inputs
status: Exploring
date: 2026-10-01
see_also: [[ADR-007]], [[EXPL-001]]
---

# EXPL-004: CrabJar Exhaust as Game Controller Inputs

## Status

Exploring. This examines the idea of treating CrabJar's structured output (exhaust) as deterministic controller inputs to a game engine — enabling replayable, auditable agent behavior in simulated environments.

## The Core Idea

From the ChatGPT conversation: speedrunners use emulators with savestates and rewind to optimize their input sequences. They're not changing the game code — they're finding the right sequence of controller inputs that achieves the goal.

CrabJar already produces structured JSON exhaust describing its actions. What if this exhaust could be fed directly into a game engine as controller inputs? The same way a TAS (Tool-Assisted Speedrun) works — you give it an input sequence, it plays back perfectly deterministically.

```pseudocode
// CrabJar decides on action
let decision = crabjar_decide(observation);
// { action: "move_up", confidence: 0.95, reasoning: "path_clear" }

// Translate to game input
let input = translate_to_game_input(decision);
// { frame: 142, button: UP, duration: 3 }

// Feed to emulator/game engine
emulator.inject_input(input);
```

This creates a **deterministic mapping** from CrabJar's decision-making process to observable behavior in a simulated environment.

## Why This Matters

### 1. Testing and Validation
Feed the same exhaust sequence into a game engine repeatedly — it should always produce the same outcome. This validates both:
- CrabJar's decision consistency (same input → same output)
- The game engine's determinism (same inputs → same state transitions)

### 2. Training Data Generation
Generate massive amounts of labeled training data by running CrabJar in simulated environments:

```pseudocode
for (let i = 0; i < 1000; i++) {
    let env = create_environment();
    let observation = env.observe();
    let decision = crabjar_decide(observation);
    let outcome = env.step(decision.action);
    
    training_data.push({
        observation,
        action: decision.action,
        reward: outcome.reward,
        done: outcome.done
    });
}
```

### 3. The "Speedrunner" Loop Formalized
The ChatGPT conversation identified the speedrunner pattern: checkpoint → act → observe → evaluate → branch. This maps directly to a game engine integration:

```pseudocode
// Save state (checkpoint)
let save_slot = emulator.save_state();

// Inject CrabJar's decision as input
emulator.inject_input(translate(decision));

// Run until meaningful outcome or timeout
let result = emulator.run_until(checkpoint_reached || timeout);

// Evaluate
if (result.success) {
    // Keep going or record success
} else {
    // Restore checkpoint and try different input
    emulator.restore_state(save_slot);
    let alt_decision = crabjar_decide_alternative(observation);
    // ... retry
}
```

### 4. Emergent Behavior Testing
Run CrabJar in novel game environments to see what kinds of strategies emerge:
- Can it learn to solve puzzles?
- Does it develop efficient navigation patterns?
- How does it handle uncertainty and partial information?

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
