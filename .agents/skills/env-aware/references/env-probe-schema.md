# Env Probe Schema

## Probe types

| Probe | Output | Purpose |
|---|---|---|
| `whoami` | user name | identify executing user |
| `whatami` | hostname | identify machine |
| `whereami` | cwd | identify working directory |
| `hardware` | cpu, cores, mem, disk | hardware capacity for OOM/disk/CPU sensitivity |
| `architecture` | arch, os, kernel | platform constraints |
| `full` | all above | complete environment snapshot |

## Output format

JSON with fields:
- `user`, `hostname`, `cwd`
- `cpu`, `cores`, `mem_total`, `disk_available`
- `arch`, `os`, `kernel`

## Use cases

- OOM risk assessment (mem_total vs task memory requirements)
- disk space check (disk_available vs task file requirements)
- CPU load assessment (cores vs parallel task count)
- platform-specific constraint checking (arch, os)
