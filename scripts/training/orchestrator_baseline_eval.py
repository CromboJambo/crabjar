#!/usr/bin/env python3
"""
Orchestrator Capability Baseline Evaluation
Tests Qwen3.5-9B (already loaded on ftw3 at :34159) on task decomposition,
skill boundary recognition, and dependency understanding before fine-tuning.

Uses the running llama-server's OpenAI-compatible API instead of loading a model locally.
"""

import json
import requests

API_URL = "http://localhost:34159/v1/chat/completions"
HEADERS = {"Content-Type": "application/json"}

def generate(prompt):
    payload = {
        "model": "qwen3.5-9b",
        "messages": [
            {"role": "system", "content": "You are a technical orchestrator that breaks down tasks and recognizes skill boundaries."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 256
    }
    response = requests.post(API_URL, headers=HEADERS, json=payload)
    response.raise_for_status()
    return response.json()["choices"][0]["message"]["content"].strip()

# Test cases across three orchestration capabilities
test_cases = [
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

print("Running evaluation against Qwen3.5-9B at localhost:34159...")
results = []

for case in test_cases:
    print(f"\n[{case['id']}] {case['capability']}")
    response = generate(case["prompt"])
    
    score = {"response_length": len(response)}
    
    if case["scoring"] == "multiple_steps":
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
print("EVALUATION SUMMARY (Qwen3.5-9B Baseline)")
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
