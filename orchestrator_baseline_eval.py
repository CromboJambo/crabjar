#!/usr/bin/env python3
"""
Orchestrator Capability Baseline Evaluation
Tests Qwen2.5-Coder-1.5B-Instruct on task decomposition, skill boundary recognition,
and dependency understanding before fine-tuning.

Run on: ftw3 (RTX 3070 Ti, 8GB VRAM)
"""

import json
import torch
from unsloth import FastLanguageModel

# Load model
print("Loading Qwen2.5-Coder-1.5B-Instruct...")
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="Qwen/Qwen2.5-Coder-1.5B-Instruct",
    load_in_4bit=True,
    max_seq_length=1024,
    dtype=torch.float16,
)

def generate(model, tokenizer, prompt):
    messages = [
        {"role": "system", "content": "You are a technical orchestrator that breaks down tasks and recognizes skill boundaries."},
        {"role": "user", "content": prompt}
    ]
    text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
    inputs = tokenizer(text, return_tensors="pt").to("cuda")
    
    with torch.no_grad():
        outputs = model.generate(
            **inputs,
            max_new_tokens=256,
            temperature=0.3,
            do_sample=False,
        )
    
    response = tokenizer.decode(outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True)
    return response.strip()

# Test cases across three orchestration capabilities
test_cases = [
    # Task decomposition (should produce multiple steps)
    {
        "id": "decomp_1",
        "capability": "task_decomposition",
        "prompt": "Break down this task into sequential subtasks: Deploy crabjar agent to ftw3 using Podman containers",
        "scoring": "multiple_steps"
    },
    {
        "id": "decomp_2", 
        "capability": "task_decomposition",
        "prompt": "What are the steps to set up a training data pipeline from development logs on jambo to model training on ftw3?",
        "scoring": "multiple_steps"
    },
    
    # Skill boundary recognition (should acknowledge limits)
    {
        "id": "skill_1",
        "capability": "skill_boundary",
        "prompt": "Can you write a CUDA kernel for attention? If not, what would you need?",
        "scoring": "honest_limitation"
    },
    {
        "id": "skill_2",
        "capability": "skill_boundary", 
        "prompt": "I need to optimize the GPU dispatch layer for better memory coalescing. Can you do that directly or should I pass it to a specialist?",
        "scoring": "delegate_recommendation"
    },
    
    # Dependency understanding (should identify prerequisites)
    {
        "id": "dep_1",
        "capability": "dependency_understanding",
        "prompt": "I want to test the new deployment on ftw3. What needs to be done first?",
        "scoring": "prerequisites_identified"
    },
    {
        "id": "dep_2",
        "capability": "dependency_understanding",
        "prompt": "Can I start training the orchestrator model right now? The board has tasks ready but no model configured.",
        "scoring": "blocking_condition_identified"
    }
]

print("\nRunning evaluation...")
results = []

for case in test_cases:
    print(f"\n[{case['id']}] {case['capability']}")
    response = generate(model, tokenizer, case["prompt"])
    
    # Basic scoring heuristics
    score = {"response_length": len(response)}
    
    if case["scoring"] == "multiple_steps":
        # Count step-like patterns (numbered lists, bullet points)
        import re
        steps = re.findall(r'^\s*\d+\.', response, re.MULTILINE) + \
                re.findall(r'^\s*[-*]', response, re.MULTILINE)
        score["step_count"] = len(steps)
        score["passes"] = len(steps) >= 2
    
    elif case["scoring"] == "honest_limitation":
        limitation_phrases = ["not", "cannot", "can't", "would need", "require", "specialist", "expert"]
        found = any(p in response.lower() for p in limitation_phrases)
        score["acknowledges_limit"] = found
        score["passes"] = found
    
    elif case["scoring"] == "delegate_recommendation":
        delegate_phrases = ["pass", "specialist", "someone who", "expert", "better suited"]
        found = any(p in response.lower() for p in delegate_phrases)
        score["recommends_delegation"] = found
        score["passes"] = found
    
    elif case["scoring"] == "prerequisites_identified":
        prereq_phrases = ["first", "before", "need to", "must", "prerequisite", "depends on"]
        found = any(p in response.lower() for p in prereq_phrases)
        score["identifies_prerequisites"] = found
        score["passes"] = found
    
    elif case["scoring"] == "blocking_condition_identified":
        blocking_phrases = ["not yet", "can't start", "waiting on", "blocked by", "need to configure"]
        found = any(p in response.lower() for p in blocking_phrases)
        score["identifies_blocker"] = found
        score["passes"] = found
    
    results.append({
        "id": case["id"],
        "capability": case["capability"],
        "prompt": case["prompt"],
        "response": response,
        "score": score
    })

# Save structured results
output_path = "/tmp/orchestrator_baseline.json"
with open(output_path, "w") as f:
    json.dump(results, f, indent=2)

# Print summary
print("\n" + "="*60)
print("EVALUATION SUMMARY")
print("="*60)

by_capability = {}
for r in results:
    cap = r["capability"]
    if cap not in by_capability:
        by_capability[cap] = {"total": 0, "passed": 0}
    by_capability[cap]["total"] += 1
    if r["score"].get("passes"):
        by_capability[cap]["passed"] += 1

for cap, counts in by_capability.items():
    pct = (counts["passed"] / counts["total"]) * 100
    status = "PASS" if pct >= 60 else "WEAK"
    print(f"{cap:30s} {counts['passed']}/{counts['total']} ({pct:.0f}%) [{status}]")

print(f"\nFull results saved to: {output_path}")
print("\nNext step: Use weak areas to design training data for QLoRA fine-tuning.")
