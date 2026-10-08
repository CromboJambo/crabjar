# Orchestrator Fine-Tuning Pipeline

This directory contains the QLoRA fine-tuning pipeline for the crabjar orchestrator model. The goal is to improve task decomposition, skill boundary recognition, and dependency understanding through targeted training data.

## Architecture

```hemlock
Development logs → Data collection → Filtering/labeling → Training dataset
                                                          ↓
                                                    QLoRA fine-tuning
                                                          ↓
                                                   Orchestrator adapter
                                                          ↓
                                                  Deployed orchestrator
```

## Components

### `generate_orchestrator_training_data.py`
Generates synthetic training examples targeting specific capability areas:
- **Skill boundary recognition** (6 examples): Teaching the model when to delegate vs handle directly
- **Dependency understanding** (8 examples): Prerequisites, blocking conditions, task sequencing
- **Task decomposition** (2 examples): Reinforcing existing capability
- **Routing decisions** (3 examples): Matching tasks to appropriate agent profiles

Output: JSONL file in chat completions format for Unsloth QLoRA training.

### `train_orchestrator.py`
Runs QLoRA fine-tuning using transformers directly (no Unsloth dependency). Trains Qwen2.5-Coder-1.5B-Instruct on the generated dataset with:
- 4-bit quantization (NF4) for memory efficiency
- LoRA adapters (r=16, alpha=32) on attention projections
- 3 epochs, batch size 1, gradient accumulation 4x

Output: Trained adapter weights at `/tmp/orchestrator_adapter`

### `evaluate_finetuned.py`
Evaluates the fine-tuned model against baseline metrics using the same evaluation framework as orchestrator_baseline_eval.py.

### `orchestrator_baseline_eval.py`
Baseline evaluation of the untrained Qwen2.5-Coder-1.5B-Instruct on orchestrator capability areas:
- Task decomposition (baseline: 100%)
- Skill boundary recognition (baseline: 0%)
- Dependency understanding (baseline: 0%)

## Usage

```bash
# Generate training data
python3 generate_orchestrator_training_data.py

# Run baseline evaluation
python3 orchestrator_baseline_eval.py

# Fine-tune the model
python3 train_orchestrator.py

# Evaluate fine-tuned model
python3 evaluate_finetuned.py
```

## Results

| Capability | Baseline | After Training | Target |
|------------|----------|----------------|--------|
| Task decomposition | 100% | - | Maintain |
| Skill boundary recognition | 0% | - | >70% |
| Dependency understanding | 0% | - | >60% |

## Future Work

- Collect real training data from jambo development sessions
- Implement automated filtering and labeling pipeline
- Scale up dataset size (currently only ~20 synthetic examples)
- Add evaluation metrics for routing accuracy