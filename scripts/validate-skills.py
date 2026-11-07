#!/usr/bin/env python3
"""Validate crabjar skills against documentation standards."""

import os
import sys
import yaml


def validate_skill(skill_dir):
    """Validate a single skill directory."""
    errors = []
    warnings = []

    # Check for SKILL.md
    skill_md_path = os.path.join(skill_dir, "SKILL.md")
    if not os.path.exists(skill_md_path):
        errors.append("Missing SKILL.md file")
        return errors, warnings

    with open(skill_md_path, "r") as f:
        content = f.read()

    # Parse YAML frontmatter
    try:
        parts = content.split("---", 2)
        if len(parts) < 3:
            errors.append("Invalid YAML frontmatter format")
            return errors, warnings
        frontmatter = yaml.safe_load(parts[1])
    except yaml.YAMLError as e:
        errors.append(f"YAML parse error: {e}")
        return errors, warnings

    # Validate required fields
    name = frontmatter.get("name") or frontmatter.get("skill_name")
    if not name:
        errors.append("Missing 'name' field")
    elif name != os.path.basename(skill_dir):
        errors.append(f"name '{name}' doesn't match directory '{os.path.basename(skill_dir)}'")

    description = frontmatter.get("description")
    if not description:
        errors.append("Missing 'description' field")
    elif not description.startswith("Use when "):
        warnings.append("Description should start with 'Use when <trigger>.'")

    category = frontmatter.get("category")
    valid_categories = [
        "github", "devops", "productivity", "research", "software-development",
        "autonomous-ai-agents", "mlops", "finance", "social-media", "note-taking",
        "terminal-browser", "video", "image-gen", "computer-use"
    ]
    if not category:
        warnings.append("Missing 'category' field")
    elif category not in valid_categories:
        warnings.append(f"Non-standard category: '{category}'")

    # Check for Core Workflow section
    if "## Core Workflow" not in content:
        warnings.append("Missing 'Core Workflow' section")

    return errors, warnings


def main():
    skills_dir = ".agents/skills"
    if not os.path.exists(skills_dir):
        print(f"Error: {skills_dir} directory not found")
        sys.exit(1)

    total_errors = 0
    total_warnings = 0
    validated = 0

    for entry in sorted(os.listdir(skills_dir)):
        skill_dir = os.path.join(skills_dir, entry)
        if not os.path.isdir(skill_dir):
            continue

        # Skip category hub directories (contain subdirectories with skills)
        subdirs = [d for d in os.listdir(skill_dir)
                  if os.path.isdir(os.path.join(skill_dir, d))]
        if subdirs:
            print(f"HUB:  {entry} ({len(subdirs)} skills)")
            continue

        errors, warnings = validate_skill(skill_dir)
        validated += 1

        if errors:
            print(f"FAIL: {entry}")
            for error in errors:
                print(f"  ERROR: {error}")
            total_errors += len(errors)
        elif warnings:
            print(f"WARN: {entry}")
            for warning in warnings:
                print(f"  WARNING: {warning}")
            total_warnings += len(warnings)
        else:
            print(f"OK:   {entry}")

    print(f"\nValidated {validated} skills: {total_errors} errors, {total_warnings} warnings")

    if total_errors > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()