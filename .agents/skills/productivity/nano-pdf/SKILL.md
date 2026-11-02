---
name: nano-pdf
description: |
  Use when editing text in existing PDFs via natural-language prompts, supporting operations like finding/replacing text, updating fields, and modifying content without recreating the document. Trigger this when the user says "edit this PDF" or needs PDF manipulation.
category: productivity
tags: [pdf, editing, documents]
---

# PDF Editing with Nano-PDF

This skill provides workflows for editing text in existing PDFs via natural-language prompts, supporting operations like finding/replacing text, updating fields, and modifying content without recreating the document from scratch.

## Core Workflow

1. **Load PDF**: Open target PDF document
2. **Parse structure**: Understand document layout and text locations
3. **Execute edits**: Apply requested text modifications
4. **Verify changes**: Confirm edits were applied correctly
5. **Save output**: Write modified PDF to specified location

## Usage Instructions

### Input Requirements
* PDF file path or URL
* Edit instructions (natural language)
* Output location (optional, defaults to overwrite)

### Example Prompts
* `/nano-pdf replace "old text" with "new text" in this PDF`
* `Edit the date on page 2 of this document`
* "Update the contact information in this PDF form"

## Reference Files
* `references/pdf-editing-guide.md`: Guide to PDF editing operations and limitations
* `references/natural-language-prompts.md`: Examples of effective edit prompts

## Bundled Scripts
None — this skill provides editing methodology and guidance.