# Project Map Format Reference

## Structure

```
${REPO_ROOT}/
├── src/
│   ├── main.rs
│   ├── project_loader.rs
│   ├── state_docs.rs
│   ├── knowledge_store/
│   └── ${REPO_ROOT}-config/
├── memory/files/
├── tests/
├── Cargo.toml
└── Justfile
```

## Format conventions

- Use tree indentation to show hierarchy
- Each entry: `├──` or `└──` prefix + name + optional comment
- Directory entries end with `/`
- File entries listed without trailing marker
- Workspace layout shows crate directories
- Core components table with role and status

## Audit rules

- Every crate directory must appear in project_map
- Every src/ subtree must appear in project_map
- Moved or renamed entries must be updated in project_map
- Missing entries flagged during audit
