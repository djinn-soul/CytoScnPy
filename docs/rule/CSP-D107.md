# CSP-D107: XPath Injection

**Vulnerability Category:** `Injection`

**Severity:** `HIGH`

## Description

This rule flags non-literal XPath expressions passed to APIs such as `lxml.etree.XPath()` or `.xpath()`. XPath has operators and predicates that can be altered when user input is concatenated into an expression.

XPath injection can expose unexpected XML nodes, bypass authorization checks, or change the meaning of a query.

## Vulnerable Code Example

```python
from lxml import etree

def find_user(tree, username):
    return tree.xpath(f"//user[name='{username}']")
```

If `username` contains quotes or XPath operators, it can change the query.

## Safe Code Example

```python
from lxml import etree

def find_user(tree, username):
    query = etree.XPath("//user[name=$name]")
    return query(tree, name=username)
```

Use XPath variables or another API that binds values separately from the query expression.

## How to Suppress a Finding

Suppress only when the expression is constrained to trusted values or when untrusted data is passed as a variable rather than interpolated into the expression.

```python
# expression is selected from a fixed allowlist.
# ignore: CSP-D107
return tree.xpath(expression)
```
