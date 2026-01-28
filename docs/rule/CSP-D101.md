# CSP-D101: SQL Injection

**Vulnerability Category:** `Injection`

**Severity:** `CRITICAL`

## Description

SQL Injection is a web security vulnerability that allows an attacker to interfere with the queries that an application makes to its database. It generally allows an attacker to view data that they are not normally able to retrieve. This might include data belonging to other users, or any other data that the application itself is able to access. In many cases, an attacker can modify or delete this data, causing persistent changes to the application's content or behavior.

This rule detects when a SQL query is constructed using unsafe string formatting (f-strings, `.format()`, or `%` formatting) with user-controlled input.

## Vulnerable Code Example

```python
import sqlite3

# Connect to the database
conn = sqlite3.connect('example.db')
cursor = conn.cursor()

# Get user input
user_id = input("Enter user ID: ")

# Vulnerable query construction
query = f"SELECT * FROM users WHERE id = {user_id}"

# Execute the query
try:
    cursor.execute(query)
    user = cursor.fetchone()
    if user:
        print(f"Welcome, {user[1]}")
    else:
        print("User not found.")
except sqlite3.Error as e:
    print(f"Database error: {e}")
finally:
    conn.close()

```
An attacker could enter `1; DROP TABLE users` as the `user_id`, and the query would become `SELECT * FROM users WHERE id = 1; DROP TABLE users`, deleting the `users` table.

## Safe Code Example

To prevent SQL injection, always use parameterized queries (also known as prepared statements). The database driver will then handle the safe substitution of the parameters.

```python
import sqlite3

# Connect to the database
conn = sqlite3.connect('example.db')
cursor = conn.cursor()

# Get user input
user_id = input("Enter user ID: ")

# Safe parameterized query
query = "SELECT * FROM users WHERE id = ?"

# Execute the query
try:
    cursor.execute(query, (user_id,))
    user = cursor.fetchone()
    if user:
        print(f"Welcome, {user[1]}")
    else:
        print("User not found.")
except sqlite3.Error as e:
    print(f"Database error: {e}")
finally:
    conn.close()
```
In this safe example, the `?` is a placeholder. The database driver ensures that the `user_id` value is treated as data, not as part of the SQL command. The placeholder style can vary depending on the database driver (`%s` is also common).

## How to Suppress a Finding

Suppressing SQL injection findings is highly discouraged. If you have rigorously sanitized the input and are certain it's safe, you can add a suppression comment.

```python
# ignore
cursor.execute(f"SELECT * FROM users WHERE id = {sanitized_id}")
```

Or, for this specific rule:

```python
# ignore: CSP-D101
cursor.execute(f"SELECT * FROM users WHERE id = {sanitized_id}")
```
