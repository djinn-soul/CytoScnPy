# CSP-D409: Use of Insecure Telnet Protocol

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `telnetlib`, Python's library for the Telnet protocol. Telnet is an obsolete and insecure remote login protocol. Similar to FTP, it transmits all data, including usernames and passwords, in cleartext.

An attacker monitoring the network can easily capture any credentials or sensitive information sent over a Telnet session. There is no confidentiality or integrity protection.

For secure remote shell access and command execution, SSH (Secure Shell) should always be used instead.

## Vulnerable Code Example

```python
import telnetlib

HOST = "telnet.example.com"
user = "myuser"
password = "my-secret-password"

try:
    # This connection is not encrypted.
    tn = telnetlib.Telnet(HOST)

    # The password is sent in cleartext.
    tn.read_until(b"login: ")
    tn.write(user.encode('ascii') + b"\n")
    if password:
        tn.read_until(b"Password: ")
        tn.write(password.encode('ascii') + b"\n")

    # All commands and their output are sent in cleartext.
    tn.write(b"ls -l\n")
    tn.write(b"exit\n")

    print(tn.read_all().decode('ascii'))

except Exception as e:
    print(f"An error occurred: {e}")
```

## Safe Code Example

Use a modern, secure protocol like SSH for remote access. The `paramiko` library is a popular choice for implementing an SSH client in Python.

```python
# First, install the library: pip install paramiko
import paramiko

hostname = 'ssh.example.com'
port = 22
username = 'myuser'
password = 'my-secret-password' # Or preferably, use key-based authentication

try:
    # Create an SSH client
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy()) # Note: Be cautious with AutoAddPolicy in production

    # The entire session is encrypted.
    client.connect(hostname, port=port, username=username, password=password)

    print("SSH connection successful.")

    # Execute a command securely
    stdin, stdout, stderr = client.exec_command('ls -l')

    # Print the output
    print(stdout.read().decode())

    client.close()

except Exception as e:
    print(f"An error occurred: {e}")
```

## How to Suppress a Finding

Using Telnet is strongly discouraged. Its use should be restricted to interacting with legacy hardware on a completely isolated and trusted network where sniffing is not possible.

```python
import telnetlib

# This connects to a legacy device on a trusted, isolated network.
# The risk has been assessed and accepted.
# ignore
tn = telnetlib.Telnet(LEGACY_DEVICE_IP)
```

Or, for this specific rule:

```python
# ignore: CSP-D409
tn = telnetlib.Telnet(LEGACY_DEVICE_IP)
```
