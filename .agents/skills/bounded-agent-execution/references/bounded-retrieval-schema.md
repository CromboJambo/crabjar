# Bounded Result Retrieval Schema

Patterns from DeepSeek-TUI handle_read tool for bounded retrieval from large
structured results.

---

## Result Types

| Type | Handle | Projections |
|---|---|---|
| sub-agent transcript | var_handle | slice, range, count, JSONPath |
| RLM final | var_handle | slice, range, count, JSONPath |
| tool artifact | var_handle | slice, range, count |
| analysis corpus | var_handle | JSONPath, chunk |

---

## Projection Operations

### slice

Return a contiguous slice of results.

```
handle_read <handle> slice <start> <end>
```

### range

Return results within a token range.

```
handle_read <handle> range <min_tokens> <max_tokens>
```

### count

Return count of results matching criteria.

```
handle_read <handle> count <criteria>
```

### JSONPath

Return results projected via JSONPath expression.

```
handle_read <handle> jsonpath <expression>
```

### chunk

Return results split into chunks for processing.

```
handle_read <handle> chunk <chunk_size>
```

---

## Threshold Configuration

| Setting | Default | Purpose |
|---|---|---|
| large_output_threshold_tokens | 4096 | Global routing threshold |
| exec_shell threshold | 2048 | Shell output synthesised aggressively |
| grep_files threshold | 2048 | Search results routing |
| web_search threshold | 8192 | Web results can be large |

### Per-Tool Override

Add `raw = true` to any tool call to bypass routing for that invocation.

---

## Workshop Variable

Large outputs routed through synthesis sub-agent are stored in workshop variable
`last_tool_result`. Parent can call `promote_to_context` later if it needs full
content.

---

*End of schema.*

</content>