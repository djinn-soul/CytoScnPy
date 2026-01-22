# CSP-D406: Use of Insecure FTP Protocol

**Vulnerability Category:** `Network`

**Severity:** `MEDIUM`

## Description

This rule flags the use of `ftplib`, Python's library for interacting with the File Transfer Protocol (FTP). FTP is an old and insecure protocol that transmits all data, including usernames and passwords, in cleartext.

Anyone who can monitor the network traffic between the client and the server can easily capture these credentials and gain unauthorized access to the FTP server. They can also read or modify any files being transferred.

Due to its lack of encryption, FTP should not be used. Secure alternatives like SFTP (SSH File Transfer Protocol) or FTPS (FTP over SSL/TLS) should be used instead.

## Vulnerable Code Example

```python
from ftplib import FTP

try:
    # The password 'my-secret-password' is sent in cleartext over the network.
    ftp = FTP('ftp.example.com')
    ftp.login('myuser', 'my-secret-password')

    print("FTP login successful.")

    # Any files transferred will also be in cleartext.
    with open('file.txt', 'rb') as f:
        ftp.storbinary('STOR file.txt', f)

    ftp.quit()
except Exception as e:
    print(f"An error occurred: {e}")
```

## Safe Code Example

Use a library that supports a secure file transfer protocol.

### Safe Example with SFTP (`paramiko`)

SFTP is a completely different protocol that runs over SSH and provides strong encryption and authentication.

```python
# First, install the library: pip install paramiko
import paramiko

hostname = 'sftp.example.com'
port = 22
username = 'myuser'
password = 'my-secret-password'

try:
    transport = paramiko.Transport((hostname, port))
    transport.connect(username=username, password=password)

    sftp = paramiko.SFTPClient.from_transport(transport)

    print("SFTP connection successful.")

    # The connection and file transfer are encrypted.
    sftp.put('local_file.txt', 'remote_file.txt')

    sftp.close()
    transport.close()
except Exception as e:
    print(f"An error occurred: {e}")
```

### Safe Example with FTPS (`ftplib`)

FTPS is an extension of FTP that adds support for TLS/SSL encryption. `ftplib` supports this via the `FTP_TLS` class.

```python
from ftplib import FTP_TLS

try:
    # Use FTP_TLS for an encrypted connection
    ftps = FTP_TLS('ftps.example.com')
    ftps.login('myuser', 'my-secret-password')
    ftps.prot_p() # Switch to data protection mode for encrypted file transfers

    print("FTPS login successful.")

    with open('file.txt', 'rb') as f:
        ftps.storbinary('STOR file.txt', f)

    ftps.quit()
except Exception as e:
    print(f"An error occurred: {e}")
```

## How to Suppress a Finding

Using FTP is strongly discouraged. You should only suppress this if you are connecting to an anonymous, public FTP server to download non-sensitive data, or if you are on a fully trusted, isolated network where traffic sniffing is not a concern.

```python
from ftplib import FTP

# Connecting to a public, anonymous FTP server. No credentials are sent.
# ignore
ftp = FTP('ftp.public-archive.org')
ftp.login() # Anonymous login
# ... download public files ...
ftp.quit()
```

Or, for this specific rule:

```python
# ignore: CSP-D406
ftp = FTP('ftp.public-archive.org')
```
