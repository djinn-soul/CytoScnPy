# CSP-D102: Raw SQL String with SQLAlchemy or Pandas

**Vulnerability Category:** `Injection`

**Severity:** `CRITICAL`

## Description

This rule is a specific variant of SQL Injection ([CSP-D101](./CSP-D101.md)) that applies to the use of raw SQL strings in higher-level libraries like SQLAlchemy and pandas. Functions like `sqlalchemy.text()` and `pandas.read_sql_query()` can execute raw SQL. If user input is incorporated into these queries using string formatting, it creates a SQL injection vulnerability.

Even when using powerful libraries, falling back to raw SQL with string concatenation re-introduces the same risks as building queries manually.

## Vulnerable Code Example (SQLAlchemy)

```python
from sqlalchemy import create_engine, text
import os

engine = create_engine("sqlite:///example.db")
user_name = input("Enter username: ")

# The user input is directly embedded in the SQL string
# An attacker could enter: "' OR '1'='1"
with engine.connect() as connection:
    query = text(f"SELECT * FROM users WHERE name = '{user_name}'")
    result = connection.execute(query).fetchall()
    for row in result:
        print(row)
```

## Vulnerable Code Example (Pandas)

```python
import pandas as pd
from sqlalchemy import create_engine

engine = create_engine("sqlite:///example.db")
table_name = input("Enter table to query: ")

# The table name is coming from user input and is not sanitized
# An attacker could inject SQL here. E.g., "users; DROP TABLE users"
df = pd.read_sql_query(f"SELECT * FROM {table_name}", engine)
print(df.head())
```

## Safe Code Example (SQLAlchemy)

SQLAlchemy's `text()` construct supports bound parameters to safely pass data into the query.

```python
from sqlalchemy import create_engine, text
import os

engine = create_engine("sqlite:///example.db")
user_name = input("Enter username: ")

# Use bound parameters (:name) to safely pass the data
with engine.connect() as connection:
    query = text("SELECT * FROM users WHERE name = :name")
    result = connection.execute(query, {"name": user_name}).fetchall()
    for row in result:
        print(row)
```

## Safe Code Example (Pandas)

Pandas' `read_sql_query` also supports parameters.

```python
import pandas as pd
from sqlalchemy import create_engine

engine = create_engine("sqlite:///example.db")
user_id = input("Enter user ID: ")

# The user input is passed safely as a parameter
df = pd.read_sql_query(
    "SELECT * FROM users WHERE id = ?",
    engine,
    params=(user_id,)
)
print(df.head())
```

## How to Suppress a Finding

This is a critical vulnerability and should not be suppressed. If you have a legitimate reason and have validated the input, you can use a suppression comment.

```python
# ignore
df = pd.read_sql_query(f"SELECT * FROM {validated_table}", engine)
```

Or, for this specific rule:

```python
# ignore: CSP-D102
query = text(f"SELECT * FROM users WHERE name = '{validated_name}'")
```
