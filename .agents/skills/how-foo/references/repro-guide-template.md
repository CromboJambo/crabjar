# Reproduction Guide Template

## Required fields

- **Source**: directory path + project description
- **Reproduction Steps**: clone/install/build commands, usage commands
- **Key Dependencies**: table of Cargo.toml deps (crate, version, purpose)
- **MSRV**: minimum supported Rust version
- **Release Profile**: opt-level, lto settings from Cargo.toml
- **Notes**: edge cases, platform restrictions, branch info

## When to skip

- Directory is a local project with no upstream repo (no clone step needed)
- Directory contains no Cargo.toml or README (insufficient data)
- User explicitly says "don't write a file" or "just tell me"
