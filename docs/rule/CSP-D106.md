# CSP-D106: LDAP Injection

**Vulnerability Category:** `Injection`

**Severity:** `HIGH`

## Description

This rule flags non-literal LDAP filter strings passed to LDAP search APIs. LDAP filters have their own expression syntax, so interpolating user input into a filter can let an attacker alter the query and bypass access controls or enumerate directory entries.

The rule covers common `python-ldap` and `ldap3` search APIs, including positional filter arguments and named filter arguments such as `filterstr` or `search_filter`.

## Vulnerable Code Example

```python
import ldap

def find_user(conn, username):
    query = f"(uid={username})"
    return conn.search_s("ou=people,dc=example,dc=com", ldap.SCOPE_SUBTREE, query)
```

An attacker can provide input like `*)(|(uid=*))` to change the intended filter.

## Safe Code Example

```python
import ldap
from ldap.filter import escape_filter_chars

def find_user(conn, username):
    safe_username = escape_filter_chars(username)
    query = f"(uid={safe_username})"
    return conn.search_s("ou=people,dc=example,dc=com", ldap.SCOPE_SUBTREE, query)
```

Escape untrusted values before embedding them in LDAP filters. Keep the filter structure static.

## How to Suppress a Finding

Suppress only when the filter string is assembled exclusively from trusted, fixed fragments or from values escaped with an LDAP filter escaping API.

```python
# username is escaped before being inserted into the filter.
# ignore: CSP-D106
conn.search_s(base, scope, query)
```
