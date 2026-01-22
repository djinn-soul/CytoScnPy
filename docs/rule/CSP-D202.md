# CSP-D202: Unsafe YAML Loading

**Vulnerability Category:** `Deserialization`

**Severity:** `HIGH`

## Description

This rule flags the use of `yaml.load()` from the PyYAML library without specifying the `Loader` parameter. The default loader, `FullLoader` (and the legacy `UnsafeLoader`), can execute arbitrary Python code if it encounters a specially crafted YAML document. This creates a severe remote code execution vulnerability if the YAML content comes from an untrusted source.

## Vulnerable Code Example

```python
import yaml
import os

# An attacker provides a malicious YAML string.
# The '!!python/object/apply:os.system' tag instructs PyYAML to execute a command.
malicious_yaml = "!!python/object/apply:os.system ['rm -rf /']"

# This line will execute os.system('rm -rf /')
# In older PyYAML versions, yaml.load(malicious_yaml) is enough.
# In newer versions, this requires yaml.load(malicious_yaml, Loader=yaml.FullLoader)
# Both are dangerous.
document = yaml.load(malicious_yaml, Loader=yaml.FullLoader)
```

## Safe Code Example

To safely load YAML documents, always use `yaml.safe_load()` or explicitly specify the `Loader=yaml.SafeLoader` argument. The safe loader can parse standard YAML tags but restricts it to simple data types like lists and dictionaries, preventing code execution.

```python
import yaml

safe_yaml = """
- user: alice
  role: admin
- user: bob
  role: guest
"""

malicious_yaml = "!!python/object/apply:os.system ['rm -rf /']"

# Using yaml.safe_load() is the recommended approach
document = yaml.safe_load(safe_yaml)
print(document)

try:
    # This will now raise an exception instead of executing code.
    yaml.safe_load(malicious_yaml)
except yaml.constructor.ConstructorError as e:
    print(f"Blocked a potential YAML deserialization attack: {e}")
```

You can also use `Loader=yaml.SafeLoader` explicitly:
```python
document = yaml.load(safe_yaml, Loader=yaml.SafeLoader)
```

## How to Suppress a Finding

If you are loading a YAML file that is fully trusted (e.g., an internal configuration file that is not modified by user input) and you require the full loading capabilities, you can suppress this finding. However, using `safe_load` is strongly preferred in almost all situations.

```python
# This config is trusted and requires advanced tags.
# ignore
config = yaml.load(trusted_config_file, Loader=yaml.FullLoader)
```

Or, for this specific rule:

```python
# ignore: CSP-D202
config = yaml.load(trusted_config_file, Loader=yaml.FullLoader)
```
