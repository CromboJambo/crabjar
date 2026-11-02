---
name: ocr-and-documents
description: |
  Use when extracting text from PDFs, scanned documents, and images using OCR tools like pymupdf and marker-pdf for document processing workflows. Trigger this when the user says "extract text from this PDF" or needs document processing.
category: productivity
tags: [ocr, pdf, documents]
---

# OCR and Document Processing

This skill provides workflows for extracting text from PDFs, scanned documents, and images using OCR tools like pymupdf and marker-pdf for comprehensive document processing workflows.

## Core Workflow

1. **Identify document type**: Determine if document is digital PDF, scan, or image
2. **Select appropriate tool**: Choose between pymupdf (digital) and marker-pdf (OCR)
3. **Extract text**: Run extraction with appropriate settings
4. **Clean output**: Remove artifacts and format extracted text
5. **Process results**: Use extracted text for downstream operations

## Usage Instructions

### Input Requirements
* Document file path or URL
* Document type (if known): digital PDF, scan, image
* Output format preference (optional)

### Example Prompts
* `/ocr-and-documents extract text from this scanned invoice`
* `Process this PDF and get the text content`
* "Extract all text from these images using OCR"

## Reference Files
* `references/ocr-tools-comparison.md`: Comparison of available OCR tools
* `references/document-processing-workflows.md`: Common document processing workflows

## Bundled Scripts
None — this skill provides processing methodology and guidance.