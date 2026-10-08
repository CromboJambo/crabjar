#!/usr/bin/env python3
"""
QLoRA fine-tuning using transformers directly (no Unsloth) due to Python 3.14 compatibility.
Trains Qwen2.5-Coder-1.5B-Instruct on orchestrator capability examples.
"""

import json
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig, TrainingArguments, Trainer
from peft import LoraConfig, get_peft_model, TaskType
from datasets import Dataset

# Load base model in 4-bit quantization
print("Loading Qwen2.5-Coder-1.5B-Instruct...")

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
tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-1.5B-Instruct")

# Add LoRA adapters for instruction tuning
peft_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    inference_mode=False,
    r=16,
    lora_alpha=32,
    lora_dropout=0.05,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
)

model = get_peft_model(model, peft_config)
model.print_trainable_parameters()

# Load training data
print("Loading training data...")
examples = []
with open("/home/crabjar-agent/orchestrator_training_data.jsonl", "r") as f:
    for line in f:
        examples.append(json.loads(line))

print(f"Loaded {len(examples)} training examples")

# Format for instruction tuning
def format_example(example):
    messages = [
        {"role": "system", "content": "You are a technical orchestrator that breaks down tasks and recognizes skill boundaries."},
        {"role": "user", "content": example["instruction"]},
        {"role": "assistant", "content": example["output"]}
    ]
    return tokenizer.apply_chat_template(messages, tokenize=False)

formatted_texts = [format_example(ex) for ex in examples]

# Tokenize
inputs = tokenizer(
    formatted_texts,
    padding=True,
    truncation=True,
    max_length=2048,
    return_tensors="pt",
)

# For causal LM training, labels should be a copy of input_ids
inputs["labels"] = inputs["input_ids"].clone()

# Training configuration
training_args = TrainingArguments(
    output_dir="/tmp/orchestrator_adapter",
    num_train_epochs=3,
    per_device_train_batch_size=1,
    gradient_accumulation_steps=4,
    learning_rate=2e-5,
    fp16=torch.cuda.is_available(),
    logging_steps=1,
)

trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=Dataset.from_dict(inputs),
)

print("Starting training...")
trainer.train()

# Save adapter
trainer.save_model("/tmp/orchestrator_adapter")
tokenizer.save_pretrained("/tmp/orchestrator_adapter")

print("Training complete. Adapter saved to /tmp/orchestrator_adapter")
