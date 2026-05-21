# Perturbation Schema

## PerturbationSet

| Field | Type | Purpose |
|---|---|---|
| `perturbations` | Vec<Perturbation> | bounded set of reachable worst-case states |
| `bound` | f64 | mitigable_count / (mitigable + unmitigable), clamped 0.0-1.0 |

### Methods

| Method | Return | Purpose |
|---|---|---|
| `perturbations()` | &[Perturbation] | expose the bounded set |
| `bound()` | f64 | expose mitigation ratio |
| `max_severity()` | f64 | maximum severity across the set |
| `has_unmitigable()` | bool | any perturbation without undo path |
| `mitigable_count()` | usize | count of mitigable perturbations |
| `unmitigable_count()` | usize | count of unmitigable perturbations |

## Perturbation

| Field | Type | Purpose |
|---|---|---|
| `kind` | PerturbationKind | type of perturbation |
| `severity` | f64 | impact weight (0.0-1.0) |
| `description` | String | what this perturbation affects |
| `mitigable` | bool | has undo path / mitigation |

### Severity defaults

| Kind | Default Severity |
|---|---|
| UndoPath | 0.25 |
| ChecksumTarget | 0.20 |
| CheckpointTarget | 0.20 |
| FlightRecorderTarget | 0.15 |
| DataIntegrityTarget | 0.20 |
| NoUndoPath | 1.0 |

## PerturbationKind

| Value | Meaning |
|---|---|
| UndoPath | explicit rollback command available |
| ChecksumTarget | file integrity verification available |
| CheckpointTarget | session checkpoint available |
| FlightRecorderTarget | traceability logging available |
| DataIntegrityTarget | data integrity verification available |
| NoUndoPath | no undo paths detected |
| NoChecksums | no checksum targets |
| NoCheckpoint | no checkpoint targets |
| NoFlightRecorder | no flight recorder targets |
| DataCorruption | data integrity compromised |

## Bound Calculation

```
bound = mitigable_count / (mitigable_count + unmitigable_count)
```

- `mitigable_count == 0` → bound = 0.0 (fully unmitigable)
- `unmitigable_count == 0` → bound = 1.0 (fully mitigable)
- both > 0 → ratio of mitigable to total

## Risk Level Determination

| Condition | Risk Level |
|---|---|
| unmitigable > 0 AND max_severity > 0.8 AND confidence < 0.5 | Critical |
| unmitigable > 0 AND max_severity > 0.5 AND confidence < 0.6 | High |
| confidence < 0.6 OR !uncertainty_exposed OR !interruptible | Medium |
| else | Low |

## Gate Check

`gate_check_with_reversibility` accepts perturbation inputs:
- `undo_paths: Vec<String>`
- `checksum_targets: Vec<String>`
- `checkpoint_targets: Vec<String>`
- `flight_recorder_targets: Vec<String>`
- `data_integrity_targets: Vec<String>`

Replaces old parameters: `has_undo_path: bool`, `has_checksums: bool`, etc.

---

*End of schema.*