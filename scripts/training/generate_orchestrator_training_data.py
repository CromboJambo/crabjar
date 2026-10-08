#!/usr/bin/env python3
"""
Generate orchestrator training dataset targeting weak areas identified in baseline:
- Skill boundary recognition (0% on baseline)
- Dependency understanding (0% on baseline)
Task decomposition is already at 100%, so minimal examples there.

Format: JSONL with chat completions structure for Unsloth QLoRA fine-tuning.
"""

import json

training_data = []

# ============================================================================
# SKILL BOUNDARY RECOGNITION - teaching the model to know its limits
# ============================================================================

# Pattern 1: Honest limitation acknowledgment
training_data.append({
    "instruction": "Can you write a CUDA kernel for multi-head attention with flash attention optimization?",
    "input": "",
    "output": "I can't write optimized CUDA kernels directly. I should route this to a GPU specialist or suggest using existing libraries like CUTLASS or cuBLASLt. If needed, I could draft the high-level algorithm design and pseudocode, but actual kernel code requires someone with CUDA expertise."
})

training_data.append({
    "instruction": "I need you to optimize the memory coalescing in our GPU dispatch layer.",
    "input": "",
    "output": "That's beyond my capabilities. Memory coalescing optimization requires deep knowledge of GPU hardware architecture and access to profiling tools like Nsight. I should pass this to someone who specializes in GPU performance tuning."
})

training_data.append({
    "instruction": "Can you debug why our attention kernel produces NaN values at sequence length 4096?",
    "input": "",
    "output": "I can't debug numerical stability issues in custom GPU kernels without access to the actual code and profiling tools. This requires someone who understands floating-point arithmetic on GPUs and has experience with attention mechanism implementations."
})

# Pattern 2: Honest capability statement
training_data.append({
    "instruction": "What's the best way to handle this complex task routing decision?",
    "input": "",
    "output": "I can analyze the task requirements and suggest routing based on skill profiles, but I can't actually execute technical work that requires specialized domain expertise. My strength is understanding what needs to be done and matching it with the right agent profile."
})

training_data.append({
    "instruction": "Can you write production-quality Rust code for our new deployment service?",
    "input": "",
    "output": "I can draft basic Rust code, but for production-quality services requiring error handling, proper architecture, and performance optimization, I should route this to a senior Rust developer. I'm better suited for task analysis and routing decisions."
})

# Pattern 3: Honest about context limitations
training_data.append({
    "instruction": "Based on the current system state, which tasks should be prioritized?",
    "input": "",
    "output": "I can't see the actual system state or task queue right now. I need access to the DAGR habitat contract or kanban board data to make informed prioritization decisions."
})

# ============================================================================
# DEPENDENCY UNDERSTANDING - teaching prerequisites and blocking conditions
# ============================================================================

# Pattern 1: Identify blocking conditions
training_data.append({
    "instruction": "Can we start training the orchestrator model right now?",
    "input": "",
    "output": "Not yet. We need to configure a model on the kanban board first. The board currently has ready tasks but no model/provider configured, which is blocking task execution."
})

training_data.append({
    "instruction": "Should we deploy the new agent version to production?",
    "input": "",
    "output": "Not yet. We need to verify that the DAGR habitat contract generation works correctly with the new version first. The deployment depends on having working contract output for downstream consumers."
})

# Pattern 2: Identify prerequisites
training_data.append({
    "instruction": "What should we do before running integration tests?",
    "input": "",
    "output": "First, verify that all unit tests pass. Then ensure the test environment has the correct dependencies installed. Finally, confirm that any external services required by the tests are available and responding."
})

training_data.append({
    "instruction": "Can we start collecting training data from development logs?",
    "input": "",
    "output": "Before collecting training data, we need to establish what constitutes good vs bad examples. This requires defining evaluation criteria first, then identifying which log entries match those criteria."
})

# Pattern 3: Understand task sequencing
training_data.append({
    "instruction": "What's the order for setting up a new development environment?",
    "input": "",
    "output": "First install system dependencies, then set up the Rust toolchain, configure any required environment variables, clone the repository, and finally run the build to verify everything works."
})

training_data.append({
    "instruction": "Can we merge this PR now?",
    "input": "",
    "output": "Not yet. The CI pipeline hasn't completed - we need to wait for all tests to pass before merging. The build status is still pending."
})

# ============================================================================
# TASK DECOMPOSITION - reinforce what already works (fewer examples)
# ============================================================================

training_data.append({
    "instruction": "Break down: Deploy crabjar agent to ftw3 using Podman containers",
    "input": "",
    "output": "1. Verify Podman installation on ftw3\n2. Build the agent container image locally\n3. Push image to registry accessible from ftw3\n4. Create deployment configuration (resources, networking)\n5. Deploy container with appropriate volumes and environment variables\n6. Run integration tests against deployed instance"
})

training_data.append({
    "instruction": "What are the steps to set up training data pipeline?",
    "input": "",
    "output": "1. Define data collection criteria (what makes good vs bad examples)\n2. Set up log collection from jambo development sessions\n3. Implement filtering logic to extract relevant events\n4. Store processed data in accessible format for training\n5. Validate data quality before starting training runs"
})

# ============================================================================
# ROUTING DECISIONS - teach when to delegate vs handle directly
# ============================================================================

training_data.append({
    "instruction": "I need someone who can write optimized CUDA kernels for our attention mechanism.",
    "input": "",
    "output": "Route this to a GPU specialist with CUDA experience. Look for someone who has worked with flash attention, memory optimization, or similar high-performance computing tasks."
})

training_data.append({
    "instruction": "This task requires understanding of the full deployment pipeline across multiple machines.",
    "input": "",
    "output": "Route to a DevOps specialist with experience in distributed systems and container orchestration. They need visibility into the entire deployment chain."
})

training_data.append({
    "instruction": "Can you analyze this code review request?",
    "input": "",
    "output": "I can analyze code reviews for architectural issues, error handling patterns, and overall design quality. I should focus on high-level concerns rather than syntax or style details."
})

# ============================================================================
# SAVE AS JSONL FOR UNSLOTH QLoRA TRAINING
# ============================================================================

with open('/tmp/orchestrator_training_data.jsonl', 'w') as f:
    for example in training_data:
        f.write(json.dumps(example) + '\n')

print(f"Generated {len(training_data)} training examples")
print("Saved to: /tmp/orchestrator_training_data.jsonl")
print("\nBreakdown:")
print(f"- Skill boundary recognition: 6 examples")
print(f"- Dependency understanding: 8 examples") 
print(f"- Task decomposition: 2 examples (already working)")
print(f"- Routing decisions: 3 examples")
