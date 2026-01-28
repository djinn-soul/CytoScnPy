# CSP-D104: Insecure XML Parsing (XXE)

**Vulnerability Category:** `Injection`

**Severity:** `HIGH` to `MEDIUM`

## Description

Parsing XML from untrusted sources using standard library modules like `xml.etree.ElementTree`, `xml.dom.minidom`, and `xml.sax` is dangerous. These parsers are susceptible to various attacks, most notably XML External Entity (XXE) injection.

An XXE attack can allow an attacker to:
- Read arbitrary files from the local filesystem.
- Initiate network requests to internal and external systems.
- Cause a Denial of Service (DoS) by consuming all available memory or CPU (e.g., via a "billion laughs attack").

This rule flags the use of these vulnerable XML parsing libraries.

## Vulnerable Code Example

```python
import xml.etree.ElementTree as ET

# Assume untrusted_xml_string is received from a user or external source
untrusted_xml_string = """
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>
"""

# Parsing this string will cause the contents of /etc/passwd to be included
# in the parsed document, which could then be exposed.
root = ET.fromstring(untrusted_xml_string)

# The content of the file is now in the text of the root element
print(root.text)
```

## Safe Code Example

To safely parse XML, use the `defusedxml` library. It's a drop-in replacement for the standard library modules that disables the insecure operations.

```python
# First, install the library: pip install defusedxml
import defusedxml.ElementTree as ET

untrusted_xml_string = """
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>
"""

try:
    # The 'defusedxml' parser will raise an exception when it
    # encounters an external entity, preventing the attack.
    root = ET.fromstring(untrusted_xml_string)
    print(root.text)
except ET.ParseError as e:
    print(f"Blocked a potential XXE attack: {e}")

```
Simply replacing the import from `xml...` to `defusedxml...` is often enough to secure your application against XXE and other XML-based attacks.

## How to Suppress a Finding

If you are parsing a trusted XML document or have manually configured the parser to be safe, you can suppress this finding. However, using `defusedxml` is the recommended approach.

```python
import xml.etree.ElementTree as ET

# You have manually configured a parser to be safe
# ignore
root = ET.fromstring(trusted_string, parser=safe_parser)
```

Or, for this specific rule:

```python
# ignore: CSP-D104
root = ET.fromstring(trusted_string)
```
