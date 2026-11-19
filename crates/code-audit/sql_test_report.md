# Bug Bounty Report

**Target:** test_sql_injection.rs
**Date:** 2026-11-06 20:43 UTC
**Hunter:** bug-bounty-hunter v0.1.0

## Summary

Total findings: **3**
- Critical: 0
- High: 3
- Medium: 0
- Low: 0

## HIGH Findings

### 1. Potential SQL injection via string concatenation

**CWE:** CWE-89

SQL query in function 'get_user_by_id' constructed using string concatenation instead of parameterized query

**Evidence:**

- Type: code_snippet
  Location: test_sql_injection.rs:5
  ```json
{
  "file": "test_sql_injection.rs",
  "function": "get_user_by_id",
  "line": 5,
  "snippet": "let query = \"SELECT * FROM users WHERE id=\" + &id.to_string();",
  "variable": "query"
}
```

**Remediation:** Use parameterized queries with placeholders instead of string concatenation

---

### 2. Potential SQL injection via string concatenation

**CWE:** CWE-89

SQL query in function 'search_users' constructed using string concatenation instead of parameterized query

**Evidence:**

- Type: code_snippet
  Location: test_sql_injection.rs:5
  ```json
{
  "file": "test_sql_injection.rs",
  "function": "search_users",
  "line": 5,
  "snippet": "let query = \"SELECT * FROM users WHERE id=\" + &id.to_string();",
  "variable": "query"
}
```

**Remediation:** Use parameterized queries with placeholders instead of string concatenation

---

### 3. Potential SQL injection via string concatenation

**CWE:** CWE-89

SQL query in function 'delete_user' constructed using string concatenation instead of parameterized query

**Evidence:**

- Type: code_snippet
  Location: test_sql_injection.rs:5
  ```json
{
  "file": "test_sql_injection.rs",
  "function": "delete_user",
  "line": 5,
  "snippet": "let query = \"SELECT * FROM users WHERE id=\" + &id.to_string();",
  "variable": "query"
}
```

**Remediation:** Use parameterized queries with placeholders instead of string concatenation

---

## Analysis Metadata

- Tool: bug-bounty-hunter v0.1.0
- Duration: 0s

**Assumptions:**
- target was reachable during analysis

**Blind Spots:**
- runtime-only bugs not detected without execution
