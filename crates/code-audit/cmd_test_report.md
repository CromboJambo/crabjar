# Bug Bounty Report

**Target:** test_cmd_injection.rs
**Date:** 2026-11-06 20:51 UTC
**Hunter:** bug-bounty-hunter v0.1.0

## Summary

Total findings: **3**
- Critical: 0
- High: 3
- Medium: 0
- Low: 0

## HIGH Findings

### 1. Potential command injection via string concatenation

**CWE:** CWE-78

Shell command in function 'list_files' constructed using string concatenation

**Evidence:**

- Type: code_snippet
  Location: test_cmd_injection.rs:5
  ```json
{
  "file": "test_cmd_injection.rs",
  "function": "list_files",
  "line": 5,
  "snippet": "let cmd = \"ls -la \" + directory;",
  "variable": "cmd"
}
```

**Remediation:** Use execvp-style argument arrays instead of shell command strings

---

### 2. Potential command injection via string concatenation

**CWE:** CWE-78

Shell command in function 'search_files' constructed using string concatenation

**Evidence:**

- Type: code_snippet
  Location: test_cmd_injection.rs:5
  ```json
{
  "file": "test_cmd_injection.rs",
  "function": "search_files",
  "line": 5,
  "snippet": "let cmd = \"ls -la \" + directory;",
  "variable": "cmd"
}
```

**Remediation:** Use execvp-style argument arrays instead of shell command strings

---

### 3. Potential command injection via string concatenation

**CWE:** CWE-78

Shell command in function 'delete_file' constructed using string concatenation

**Evidence:**

- Type: code_snippet
  Location: test_cmd_injection.rs:5
  ```json
{
  "file": "test_cmd_injection.rs",
  "function": "delete_file",
  "line": 5,
  "snippet": "let cmd = \"ls -la \" + directory;",
  "variable": "cmd"
}
```

**Remediation:** Use execvp-style argument arrays instead of shell command strings

---

## Analysis Metadata

- Tool: bug-bounty-hunter v0.1.0
- Duration: 0s

**Assumptions:**
- target was reachable during analysis

**Blind Spots:**
- runtime-only bugs not detected without execution
