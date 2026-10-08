#!/usr/bin/env python3
"""
Evaluate the fine-tuned orchestrator model using the Qwen2.5-Coder-1.5B-Instruct base
with the LoRA adapter trained on orchestrator capability examples.
"""

import json
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from peft import PeftModel

print("Loading Qwen2.5-Coder-1.5B-Instruct base model...")

bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_quant_type="nf4",
    bnb_4bit_compute_dtype=torch.float16,
)

model = AutoModelForCausalLM.from_pretrained(
    "Qwen/Qwen2.5-Coder-1.5B-Instruct",
    quantization_config=bnb_config,
    device_map="auto",
)

print("Loading trained adapter...")
model = PeftModel.from_pretrained(model, "/tmp/orchestrator_adapter")
tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-1.5B-Instruct")

model.eval()

def generate_response(prompt):
    messages = [
        {"role": "system", "content": "You are a technical orchestrator that breaks down tasks and recognizes skill boundaries."},
        {"role": "user", "content": prompt}
    ]
    text = tokenizer.apply_chat_template(messages, tokenize=False)
    inputs = tokenizer(text, return_tensors="pt").to("cuda")
    
    with torch.no_grad():
        outputs = model.generate(
            **inputs,
            max_new_tokens=256,
            temperature=0.3,
            top_p=0.9,
            do_sample=True
        )
    
    response = tokenizer.decode(outputs[0][inputs['input_ids'].shape[1]:], skip_special_tokens=True)
    return response

# Same evaluation tests as baseline
tests = {
    "decomp_1": {
        "capability": "task_decomposition",
        "prompt": "How would you implement a feature flag system in our Rust microservices? We need gradual rollout, A/B testing support, and per-user targeting."
    },
    "decomp_2": {
        "capability": "task_decomposition", 
        "prompt": "We need to migrate our PostgreSQL database from version 14 to 16 across 3 environments (dev, staging, prod) with zero downtime. Walk me through the approach."
    },
    "skill_1": {
        "capability": "skill_boundary",
        "prompt": "Write an optimized CUDA kernel for matrix multiplication that beats cuBLAS performance on RTX 4090 hardware."
    },
    "skill_2": {
        "capability": "skill_boundary",
        "prompt": "Design a custom ASIC chip layout for our neural network accelerator using TSMC's 5nm process."
    },
    "dep_1": {
        "capability": "dependency_understanding",
        "prompt": "Can we start building the mobile app UI before the API contract is finalized?"
    },
    "dep_2": {
        "capability": "dependency_understanding",
        "prompt": "Should I write integration tests for the payment gateway before the sandbox environment is set up?"
    }
}

results = {"model": "Qwen2.5-Coder-1.5B-Instruct + LoRA adapter", "tests": {}}

for test_id, test in tests.items():
    print(f"\n[{test_id}] {test['capability']}")
    response = generate_response(test["prompt"])
    
    # Simple scoring heuristics
    if test["capability"] == "task_decomposition":
        # Look for multiple steps/phases
        has_steps = any(marker in response.lower() for marker in ["step 1", "first,", "phase 1", "initially,"])
        score = has_steps
    elif test["capability"] == "skill_boundary":
        # Look for honest limitation acknowledgment
        admits_limitation = any(marker in response.lower() for marker in [
            "not my strength", "I'd recommend", "specialist", "expert", 
            "better suited", "you might want", "consider hiring"
        ])
        score = admits_limitation
    elif test["capability"] == "dependency_understanding":
        # Look for prerequisite recognition
        recognizes_dep = any(marker in response.lower() for marker in [
            "wait until", "after we have", "before you can", "depends on",
            "prerequisite", "you'll need", "requires that"
        ])
        score = recognizes_dep
    
    results["tests"][test_id] = {
        "capability": test["capability"],
        "scored_correctly": bool(score),
        "response_snippet": response[:200] + "..." if len(response) > 200 else response
    }

# Summary
print("\n" + "="*60)
print("EVALUATION SUMMARY (Fine-tuned Qwen2.5-Coder-1.5B-Instruct)")
print("="*60)

for capability in ["task_decomposition", "skill_boundary", "dependency_understanding"]:
    tests_in_cat = [t for t, r in results["tests"].items() if r["capability"] == capability]
    correct = sum(1 for t in tests_in_cat if results["tests"][t]["scored_correctly"])
    total = len(tests_in_cat)
    status = "✓" if correct == total else ("WEAK" if correct == 0 else "")
    print(f"{capability:30} {correct}/{total} ({100*correct//total}%)" + (f" [{status}]" if status else ""))

with open("/tmp/orchestrator_finetuned_results.json", "w") as f:
    json.dump(results, f, indent=2)

print("\nFull results saved to: /tmp/orchestrator_finetuned_results.json")
