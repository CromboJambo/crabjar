# Grex Skill: Pattern Verification Engine

## 🎯 Purpose
The `grex` skill is the **Verification Pillar** of the Islet architecture. Its sole responsibility is to perform structural verification of files within the workspace against a set of predefined patterns (regex or literal). It provides machine-readable feedback to enable automated, self-correcting loops.

## 🛠️ Input Schema
To invoke this skill, the agent must provide an input object containing:

| Field | Type | Description | Required |
|---|---|---|---|
| `target_path` | `string` | The absolute path to the file or directory being verified. | Yes |
| `pattern` | `string` | The regex pattern or literal string to search for. | Yes |
| `mode` | `enum` | `EXISTS` (check if pattern is present) or `ABSENT` (ensure pattern is NOT present). | Yes |
| `context_limit` | `int` | (Optional) Max number of lines to scan from the start of the file. | No |

**Example Input:**
```json
{
  "target_path": "mirror-lab/AGENTS.md",
  "pattern": "# Repository Guidelines",
  "mode": "EXISTS"
}
```

## 📊 Output Format (Machine-Readable)
The skill MUST return a structured response in the following format to ensure downstream skills (like `pi-mono`) can parse the result without ambiguity.

| Field | Type | Description |
|---|---|---|
| `status` | `enum` | `SUCCESS` (Verification passed) or `FAILURE` (Verification failed). |
| `match_count` | `int` | The number of times the pattern was found in the target path. |
            | `line_numbers` | An array of integers representing the line numbers where matches were found. |
| `error_message` | `string` | If `status` is `FAILURE`, a brief description of why it failed (e.g., "Pattern not found"). |

**Example Success Output:**
```json
{
  "status": "SUCCESS",
  "match_count": 1,
  "line_numbers": [1],
  "error_message": null
}
```

**Example Failure Output:**
```json
{
  "status": "FAILURE",
  "match_count": 0,
  "line_numbers": [],
  "error_message": "Pattern '# Repository Guidelines' not found in target_path."
}
```

## ⚙️ Operational Constraints
- **Scope:** The skill should only operate on files within the `mirror-lab` workspace.
- **Immutability:** This skill is a *read-only* verification engine. It must NEVER modify the file it is inspecting.
- **Performance:** For large files, use the `context_limit` to prevent excessive resource consumption.