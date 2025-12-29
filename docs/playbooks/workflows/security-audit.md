# Workflow: Security Audit

Run a comprehensive security analysis of your codebase using narsil-mcp's security tools.

## The Scenario

You need to:
- Find security vulnerabilities before deployment
- Check for OWASP Top 10 and CWE Top 25 issues
- Identify hardcoded secrets
- Audit dependencies for known CVEs
- Verify license compliance

## Example Conversation

### Step 1: Quick security overview

**You:** "Run a security scan on this project"

**Claude calls:**
```
get_security_summary(repo: "myproject")
```

**Result:**
```
Security Summary for myproject:
- Critical: 2
- High: 5
- Medium: 12
- Low: 8

Top issues:
- SQL Injection (3 findings)
- Hardcoded secrets (2 findings)
- Vulnerable dependencies (2 findings)
```

**Claude responds:** "Found 27 security issues. 2 are critical - let me get details on those first..."

---

### Step 2: OWASP Top 10 scan

**You:** "Check for OWASP Top 10 vulnerabilities"

**Claude calls:**
```
check_owasp_top10(repo: "myproject")
```

**Result:**
```
A03:2021 - Injection
  - sql_injection at src/api/users.py:45 (CRITICAL)
    query = f"SELECT * FROM users WHERE id = {user_id}"
  - sql_injection at src/api/orders.py:78 (CRITICAL)
    cursor.execute(f"DELETE FROM orders WHERE id = {order_id}")

A07:2021 - Identification and Authentication Failures
  - weak_password_hash at src/auth/passwords.py:12 (HIGH)
    Using MD5 for password hashing
```

**Claude responds:** "Found 2 critical SQL injection vulnerabilities and a weak password hashing issue. The SQL injections at lines 45 and 78 use string formatting directly with user input - these need parameterized queries immediately."

---

### Step 3: Trace the injection path

**You:** "Show me how user input reaches that SQL query"

**Claude calls:**
```
trace_taint(repo: "myproject", path: "src/api/users.py", line: 45)
get_taint_sources(repo: "myproject", path: "src/api/users.py")
```

**Result:**
```
Taint flow:
  Source: request.args.get("user_id") at line 38 (USER_INPUT)
  ↓ passed to: get_user(user_id) at line 40
  ↓ used in: f"SELECT * FROM users WHERE id = {user_id}" at line 45
  Sink: cursor.execute(query) at line 46 (SQL_QUERY)
```

**Claude responds:** "The taint flow is clear:
1. User input from `request.args.get('user_id')` (line 38)
2. Passed directly to `get_user()` without validation
3. Interpolated into SQL string (line 45)
4. Executed directly

This is exploitable with payloads like `1; DROP TABLE users--`"

---

### Step 4: Get fix suggestions

**You:** "How should I fix this?"

**Claude calls:**
```
suggest_fix(repo: "myproject", path: "src/api/users.py", line: 45)
explain_vulnerability(cwe: "CWE-89")
```

**Result:**
```
Suggested fix:
  - query = "SELECT * FROM users WHERE id = %s"
  + cursor.execute(query, (user_id,))

CWE-89 (SQL Injection):
  Use parameterized queries or prepared statements.
  Never concatenate user input into SQL strings.
  Validate input types before use.
```

**Claude responds:** "Fix by using parameterized queries. Change:
```python
# Before (vulnerable)
query = f"SELECT * FROM users WHERE id = {user_id}"
cursor.execute(query)

# After (safe)
query = "SELECT * FROM users WHERE id = %s"
cursor.execute(query, (user_id,))
```"

---

### Step 5: Check for secrets

**You:** "Are there any hardcoded secrets in the code?"

**Claude calls:**
```
scan_security(repo: "myproject", ruleset: "secrets")
```

**Result:**
```
SECRETS-001 at src/config/settings.py:15 (CRITICAL)
  API_KEY = "sk-live-xxxxxxxxxxxxxxxxxxxx..."

SECRETS-003 at src/services/email.py:8 (HIGH)
  SMTP_PASSWORD = "emailpass123"

SECRETS-002 at .env.example:3 (INFO)
  DATABASE_URL = "postgres://user:example@localhost/db"
```

**Claude responds:** "Found 2 real secrets that should be removed:
1. **Critical:** Live API key in `settings.py:15`
2. **High:** SMTP password in `email.py:8`

The `.env.example` finding is informational - example values are fine."

---

### Step 6: Audit dependencies

**You:** "Check if any dependencies have known vulnerabilities"

**Claude calls:**
```
check_dependencies(repo: "myproject", severity_threshold: "medium")
```

**Result:**
```
Vulnerable dependencies found:

cryptography 3.4.6 → 41.0.0 recommended
  - CVE-2023-38325 (HIGH): Bleichenbacher timing oracle
  - CVE-2023-23931 (MEDIUM): Memory corruption

requests 2.25.1 → 2.31.0 recommended
  - CVE-2023-32681 (MEDIUM): Unintended proxy leak
```

**Claude responds:** "2 dependencies have known vulnerabilities. Both have patches available - upgrade to the recommended versions."

---

### Step 7: Find upgrade path

**You:** "What's the safest way to upgrade cryptography?"

**Claude calls:**
```
find_upgrade_path(repo: "myproject", dependency: "cryptography")
```

**Result:**
```
cryptography upgrade path:
  Current: 3.4.6

  3.4.6 → 3.4.8 (patch, no breaking changes)
    Fixes: CVE-2023-23931

  3.4.8 → 41.0.0 (major, breaking changes)
    Fixes: CVE-2023-38325
    Breaking: API changes in X509 handling

  Recommendation: Upgrade to 41.0.0 and test X509 code paths
```

---

### Step 8: License compliance

**You:** "Check for license compliance issues"

**Claude calls:**
```
check_licenses(repo: "myproject", project_license: "MIT")
```

**Result:**
```
License analysis for MIT project:

Compatible (42 packages):
  - flask (BSD-3-Clause)
  - requests (Apache-2.0)
  ...

Requires attention (2 packages):
  - PyQt5 (GPL-3.0) - Copyleft, may require source disclosure
  - mysql-connector-python (GPL-2.0) - Copyleft license

Unknown (1 package):
  - internal-lib (no license specified)
```

**Claude responds:** "Found 2 GPL-licensed packages which may conflict with MIT licensing..."

---

### Step 9: Generate SBOM

**You:** "Generate an SBOM for compliance"

**Claude calls:**
```
generate_sbom(repo: "myproject", format: "cyclonedx")
```

**Result:** Full CycloneDX SBOM JSON

**Claude responds:** "Generated CycloneDX SBOM with 45 components. This can be submitted for compliance or fed into vulnerability scanners."

## Security Scan Summary

| Check | Tools Used |
|-------|------------|
| Full security scan | `get_security_summary`, `scan_security` |
| OWASP Top 10 | `check_owasp_top10` |
| CWE Top 25 | `check_cwe_top25` |
| Injection flaws | `find_injection_vulnerabilities`, `trace_taint` |
| Secrets detection | `scan_security` with `secrets` ruleset |
| Crypto issues | `scan_security` with `crypto` ruleset |
| Dependency CVEs | `check_dependencies` |
| License compliance | `check_licenses` |
| SBOM generation | `generate_sbom` |

## Sample Prompts

```
"Run a full security audit on this project"
"Check for OWASP Top 10 vulnerabilities"
"Are there any hardcoded API keys or passwords?"
"Trace how user input reaches the database"
"Which dependencies have known CVEs?"
"Is this code vulnerable to XSS?"
"Explain this vulnerability and how to fix it"
"Generate an SBOM in CycloneDX format"
"Check for GPL license conflicts"
```

## Related Workflows

- [Fix a Bug](fix-a-bug.md) - Debug security issues
- [Understand a Codebase](understand-codebase.md) - Context before auditing
- [Code Review](code-review.md) - Security-focused code review
