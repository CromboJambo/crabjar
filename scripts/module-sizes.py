#!/usr/bin/env python3
"""Report Rust module sizes across the workspace.

Usage:
    python scripts/module-sizes.py [--threshold 500]

Fails (exit 1) if any module exceeds the threshold.
"""
import sys
import pathlib
import argparse


def main():
    parser = argparse.ArgumentParser(description="Report Rust module sizes")
    parser.add_argument("--threshold", type=int, default=500, help="Line threshold (default: 500)")
    args = parser.parse_args()

    root = pathlib.Path(__file__).resolve().parent.parent

    # Find all .rs files excluding target/
    rs_files = []
    for f in root.rglob("*.rs"):
        if "target" in f.parts:
            continue
        rs_files.append(f)

    sizes = []
    for f in rs_files:
        lines = len(f.read_text().splitlines())
        if lines > args.threshold:
            sizes.append((lines, f))

    sizes.sort(key=lambda x: -x[0])

    if sizes:
        print(f"Modules exceeding {args.threshold} lines:")
        for lines, path in sizes:
            rel = path.relative_to(root)
            print(f"  {lines:5d} {rel}")
        sys.exit(1)
    else:
        print(f"All modules under {args.threshold} lines ✅")
        sys.exit(0)


if __name__ == "__main__":
    main()
