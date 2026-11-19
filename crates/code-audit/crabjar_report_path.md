# Bug Bounty Report

**Target:** /home/crombo/projects/active/crabjar/src
**Date:** 2026-11-06 21:00 UTC
**Hunter:** bug-bounty-hunter v0.1.0

## Summary

Total findings: **7**
- Critical: 0
- High: 0
- Medium: 0
- Low: 7

## LOW Findings

### 1. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/metrics/module_sizes.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/metrics/module_sizes.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 2. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/metrics/test_count.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/metrics/test_count.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 3. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/commands/decide_criteria.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/commands/decide_criteria.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 4. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/commands/decide_criteria.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/commands/decide_criteria.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 5. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/commands/impact_assessment.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/commands/impact_assessment.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 6. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/commands/semantic.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/commands/semantic.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

### 7. complex_function

**CWE:** N/A

Function exceeds complexity threshold (many control flow statements)

**Evidence:**

- Type: source_code
  Location: /home/crombo/projects/active/crabjar/src/habitat_contract/generate.rs:1
  ```json
{
  "file": "/home/crombo/projects/active/crabjar/src/habitat_contract/generate.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Refactor complex functions into smaller, more focused functions.

---

## Analysis Metadata

- Tool: bug-bounty-hunter v0.1.0
- Duration: 0s

**Assumptions:**
- target was reachable during analysis

**Blind Spots:**
- runtime-only bugs not detected without execution
