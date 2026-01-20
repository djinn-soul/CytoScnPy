# Category 2: Injection & Logic Attacks (CSP-D1xx)

Rules in this category detect SQL injection, Cross-Site Scripting (XSS), and insecure XML processing.

| Rule ID      | Pattern                                 | Severity     | Why it's risky             | Safer alternative / Fix                  |
| :----------- | :-------------------------------------- | :----------- | :------------------------- | :--------------------------------------- |
| **CSP-D101** | `cursor.execute` (f-string/concat)      | **CRITICAL** | SQL injection (cursor)     | Use parameterized queries (`?`, `%s`)    |
| **CSP-D102** | `sqlalchemy.text`, `read_sql` (dynamic) | **CRITICAL** | SQL injection (raw)        | Use bound parameters / ORM builders      |
| **CSP-D103** | Flask/Jinja dynamic templates           | **CRITICAL** | XSS (Cross-site scripting) | Use static templates; escape content     |
| **CSP-D104** | `xml.etree`, `minidom`, `sax`, `lxml`   | HIGH / MED   | XXE / DoS                  | Use `defusedxml`                         |
| **CSP-D105** | `django.utils.safestring.mark_safe`     | **MEDIUM**   | XSS bypass                 | Avoid unless content is strictly trusted |

## In-depth: SQL Injection (CSP-D101)

SQL injection occurs when user input is directly concatenated into a SQL string.

### Dangerous Pattern

```python
query = f"SELECT * FROM users WHERE username = '{user_input}'"
cursor.execute(query) # VULNERABLE
```

### Safe Alternative

```python
query = "SELECT * FROM users WHERE username = %s"
cursor.execute(query, (user_input,)) # SAFE: Parameterized
```
