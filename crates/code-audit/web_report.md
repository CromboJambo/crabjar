# Bug Bounty Report

**Target:** https://example.com
**Date:** 2026-11-06 19:21 UTC
**Hunter:** bug-bounty-hunter v0.1.0

## Summary

Total findings: **2**
- Critical: 0
- High: 0
- Medium: 1
- Low: 1

## MEDIUM Findings

### 1. Missing Content-Security-Policy header

**CWE:** CWE-1021

No CSP header found, vulnerable to XSS and injection attacks

**Evidence:**

- Type: http_headers
  Location: https://example.com
  ```json
{
  "missing_header": "content-security-policy"
}
```

**Remediation:** Add Content-Security-Policy header with restrictive policy

---

## LOW Findings

### 1. Missing X-Frame-Options header

**CWE:** CWE-1021

Page can be framed, vulnerable to clickjacking

**Evidence:**

- Type: http_headers
  Location: https://example.com
  ```json
{
  "missing_header": "x-frame-options"
}
```

**Remediation:** Add X-Frame-Options: SAMEORIGIN or DENY header

---

## Analysis Metadata

- Tool: bug-bounty-hunter v0.1.0
- Duration: 0s

**Assumptions:**
- target was reachable during analysis

**Blind Spots:**
- runtime-only bugs not detected without execution
