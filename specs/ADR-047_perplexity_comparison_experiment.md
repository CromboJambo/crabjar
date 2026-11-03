# Spec: Multi-Agent Perplexity Comparison Experiment

## Overview

Use crabjar's conductor/worker architecture to run a distributed perplexity comparison between BPE and structural+semantic tokenization on Rust code. This validates whether our structural tokenizer produces more learnable representations for LLM training.

## Goal

Train two small language models from scratch on the same Rust corpus:
- **Baseline**: Standard BPE tokenization (HuggingFace `AutoTokenizer`)
- **Experimental**: Structural+semantic tokens via pesti-structural-tokenizer

Compare validation perplexity to determine which representation is more efficient for model learning. Lower perplexity = better tokenization.

## Architecture

```hemlock
Conductor Agent (ftw3:8091)
├── Goal: "Perplexity comparison experiment"
│   ├── Task 1: Corpus Collection (any worker)
│   │   └── Gather ~50 Rust files from pesti project
│   ├── Task 2: BPE Tokenization + Training (GPU worker)
│   │   ├── Install deps: transformers, torch, datasets
│   │   ├── Load Qwen2.5-Coder-1.5B-Instruct tokenizer
│   │   ├── Train TinyLM on BPE sequences
│   │   └── Report validation perplexity
│   ├── Task 3: Structural Tokenization + Training (GPU worker)
│   │   ├── Build pesti-structural-tokenizer binary
│   │   ├── Tokenize corpus with structural+semantic layer
│   │   ├── Train TinyLM on structural sequences
│   │   └── Report validation perplexity
│   └── Task 4: Result Aggregation (conductor)
│       ├── Compare perplexities
│       ├── Calculate improvement percentage
│       └── Write results to docs/
```

## Worker Requirements

| Node | Role | GPU | Dependencies |
|------|------|-----|--------------|
| ftw3 | Conductor + Worker | RTX 3070 Ti | Python 3.14, torch, transformers |
| jambo | Optional Worker | None | Python 3.14 (for corpus collection) |

## Task Specifications

### Task 1: Corpus Collection
- **Input**: pesti repo path (`/home/crombo/projects/active/pesti`)
- **Output**: JSONL file with ~50 Rust source files
- **Steps**:
  1. Clone pesti repo if not present
  2. Collect .rs files (exclude target/, .git/)
  3. Write to `/tmp/perplexity_exp/corpus.jsonl`
- **Worker**: Any node with git access

### Task 2: BPE Baseline Training
- **Input**: Corpus JSONL from Task 1
- **Output**: Validation perplexity score (float)
- **Steps**:
  1. Install `transformers`, `torch`, `datasets` via pip/uv
  2. Load Qwen2.5-Coder-1.5B-Instruct tokenizer
  3. Tokenize corpus with BPE
  4. Train TinyLM (same architecture as in experiments/perplexity_comparison.py) for 5 epochs
  5. Compute validation perplexity on held-out set
- **Worker**: GPU node (ftw3)

### Task 3: Structural+Semantic Training
- **Input**: Corpus JSONL from Task 1
- **Output**: Validation perplexity score (float)
- **Steps**:
  1. Build pesti-structural-tokenizer (`cargo build --release`)
  2. Tokenize corpus with `tokenize_with_semantics()` API
  3. Convert semantic tags to token IDs (use tag byte as ID, vocab size = 64)
  4. Train identical TinyLM architecture for 5 epochs
  5. Compute validation perplexity on held-out set
- **Worker**: GPU node (ftw3)

### Task 4: Result Aggregation
- **Input**: Perplexity scores from Tasks 2 and 3
- **Output**: Comparison report written to crabjar/docs/
- **Steps**:
  1. Collect results from Tasks 2 and 3
  2. Calculate improvement/degradation percentage
  3. Write structured results JSON to `/tmp/perplexity_exp/results.json`
  4. Write human-readable report to crabjar repo

## Implementation Notes

### Model Architecture (shared)
```hemlock
TinyLM:
- Embedding: vocab_size × 64
- TransformerEncoderLayer: d_model=64, nhead=2, dim_feedforward=512
- Layers: 1
- Output head: Linear(64, vocab_size)
```

### Training Config (shared)
- Epochs: 5
- Learning rate: 0.005
- Batch size: 32
- Optimizer: AdamW
- Sequence length: max 128 tokens
- Train/val split: 80/20

### Tokenization Mapping
- BPE: Use Qwen2.5-Coder tokenizer directly (vocab ~150K)
- Structural: Map pesti token types to sequential IDs 0-63, plus semantic tag bits as separate feature channel

## Success Criteria

1. Both models train successfully and converge
2. Perplexity scores are comparable in magnitude (within 1 order of magnitude)
3. Clear winner identified (one representation is measurably better)
4. Results written to persistent storage and reported to user

## Failure Modes & Mitigations

| Failure | Mitigation |
|---------|------------|
| GPU OOM on ftw3 | Reduce batch size to 16, then 8 |
| Conductor not running | Deploy conductor first (see conductor-crate.md spec) |
| pesti tokenizer build fails | Fall back to regex-based structural approximation |
| Training diverges | Reduce learning rate to 0.001 |

## Deliverables

1. `/tmp/perplexity_exp/corpus.jsonl` — collected Rust corpus
2. `/tmp/perplexity_exp/results.json` — structured comparison results
3. Crabjar repo commit with results documentation
4. Updated crabjar/docs/README.md with experiment link

## Dependencies on Existing Infrastructure

- **Conductor service**: Must be deployed to ftw3 first (see `conductor-crate.md`)
- **Worker agents**: jambo and ftw3 must have crabjar daemons running
- **Training scripts**: Reuse patterns from `crabjar/scripts/training/`
- **PESTI tokenizer**: Build from source at `/home/crombo/projects/active/pesti`

## Estimated Duration

- Corpus collection: ~1 minute
- BPE training: ~30 minutes (GPU)
- Structural training: ~30 minutes (GPU)
- Total: ~1 hour (parallelizable if 2 GPU workers available)
