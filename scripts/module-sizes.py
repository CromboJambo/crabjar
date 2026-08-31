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
    parser.add_argument(
        "--allowlist",
        default=str(pathlib.Path(__file__).resolve().parent / "module-sizes-allowlist.txt"),
        help="Path to the grandfathered-offenders allowlist (default: next to this script)",
    )
    args = parser.parse_args()

    root = pathlib.Path(__file__).resolve().parent.parent

    # Grandfathered offenders: already over the limit before the gate was
    # enforceable. The gate only fails on NEW offenders (files not listed).
    allowlist_path = pathlib.Path(args.allowlist)
    grandfathered = set()
    if allowlist_path.exists():
        grandfathered = {
            line.strip()
            for line in allowlist_path.read_text().splitlines()
            if line.strip() and not line.startswith("#")
        }

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

    # Offenders NOT on the allowlist are new violations -> fail.
    new_offenders = [
        (lines, f) for lines, f in sizes if f.relative_to(root).as_posix() not in grandfathered
    ]
    grandfathered_hits = [
        (lines, f) for lines, f in sizes if f.relative_to(root).as_posix() in grandfathered
    ]

    if new_offenders:
        print(f"NEW modules exceeding {args.threshold} lines (not grandfathered):")
        for lines, path in new_offenders:
            rel = path.relative_to(root)
            print(f"  {lines:5d} {rel}")
        print()
        print("Fix: split the module, or add it to the allowlist if it is pre-existing debt.")
        sys.exit(1)

    if grandfathered_hits:
        print(
            f"All new modules under {args.threshold} lines ✅ "
            f"({len(grandfathered_hits)} grandfathered offenders on the allowlist — shrink, don't grow)"
        )
    else:
        print(f"All modules under {args.threshold} lines ✅")
    sys.exit(0)


if __name__ == "__main__":
    main()
