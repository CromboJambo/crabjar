# Bug Bounty Report

**Target:** test_target.rs
**Date:** 2026-11-06 19:09 UTC
**Hunter:** bug-bounty-hunter v0.1.0

## Summary

Total findings: **3**
- Critical: 0
- High: 3
- Medium: 0
- Low: 0

## HIGH Findings

### 1. hardcoded_credentials

**CWE:** CWE-798

Hardcoded credentials or secrets detected in source code

**Evidence:**

- Type: source_code
  Location: test_target.rs:4
  ```json
{
  "file": "test_target.rs",
  "line": 4,
  "snippet": "let password = \"super_secret_password_123\";"
}
```

**Remediation:** Use environment variables or a secrets manager instead of hardcoding credentials.

---

### 2. hardcoded_credentials

**CWE:** CWE-798

Hardcoded credentials or secrets detected in source code

**Evidence:**

- Type: source_code
  Location: test_target.rs:5
  ```json
{
  "file": "test_target.rs",
  "line": 5,
  "snippet": "let api_key = \"sk-1234567890abcdef\";"
}
```

**Remediation:** Use environment variables or a secrets manager instead of hardcoding credentials.

---

### 3. string_concat_injection

**CWE:** CWE-89

String concatenation used in query/command construction (potential injection)

**Evidence:**

- Type: source_code
  Location: test_target.rs:1
  ```json
{
  "file": "test_target.rs",
  "line": 1,
  "snippet": ""
}
```

**Remediation:** Use parameterized queries or prepared statements instead of string concatenation.

---

## Analysis Metadata

- Tool: bug-bounty-hunter v0.1.0
- Duration: 0s

**Assumptions:**
- target was reachable during analysis

**Blind Spots:**
- runtime-only bugs not detected without execution
