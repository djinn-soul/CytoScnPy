# Extensive Security Corpus for CytoScnPy
# This file contains 50+ test cases to verify the refined security rules.

import os
import ssl
import socket
import hashlib
import requests
import httpx

# Tests for improved SSRF detection
requests.request("GET", user_input)  # Positional dynamic URL
requests.request("POST", url=user_input)  # Keyword dynamic URL
requests.request("GET", url=user_input, timeout=5)  # Mixed args + kwarg check
httpx.request("GET", url=user_input)  # httpx keyword check
import httpx
import marshal
import pickle
import xml.etree.ElementTree as ET
import lxml.etree
from jinja2 import Environment
from django.utils.safestring import mark_safe
import random
import telnetlib
import ftplib
import subprocess

# --- CSP-D001: eval ---
eval("os.remove('file')") # unsafe
eval(compile("1+1", "", "eval")) # unsafe

# --- CSP-D002: exec ---
exec("x = 1") # unsafe

# --- CSP-D003: os.system / subprocess.shell ---
cmd = "ls"
os.system(cmd) # unsafe
subprocess.call(cmd, shell=True) # unsafe
subprocess.run(cmd, shell=True) # unsafe
subprocess.Popen(cmd, shell=True) # unsafe
subprocess.run(["ls", "-l"]) # safe

# --- CSP-D004/D005/D006: Insecure Imports / Calls ---
import telnetlib # unsafe (import)
import ftplib # unsafe (import)
telnetlib.Telnet("host") # unsafe (call)
ftplib.FTP("host") # unsafe (call)

# --- CSP-D404: Hardcoded Bind ---
host = "0.0.0.0" # unsafe
BIND_ADDR = "::" # unsafe
listen_host = "0.0.0.0" # unsafe
ipv6_bind = "::" # unsafe
server_host = "0.0.0.0" # unsafe
public_host = "0.0.0.0" # unsafe

# Scoped bind checks
app.run(host="0.0.0.0") # unsafe
socket.bind(("0.0.0.0", 80)) # unsafe
s = socket.socket()
s.bind(("::", 443)) # unsafe

# Safe bind
local_host = "127.0.0.1" # safe
loopback = "::1" # safe
other_string = "0.0.0.0" # safe (if not in host/bind context, but currently rule is scoped to var names)
print("0.0.0.0") # safe (not host/bind context)

# --- CSP-D405: Request without timeout ---
requests.get("url") # unsafe
requests.post("url", data={}) # unsafe
requests.put("url", timeout=None) # unsafe
requests.patch("url", timeout=0) # unsafe
requests.delete("url", timeout=False) # unsafe
requests.options("url", timeout=0.0) # unsafe

# httpx
httpx.get("url") # unsafe
httpx.post("url", timeout=None) # unsafe
httpx.request("GET", "url") # unsafe

# Safe timeouts
requests.get("url", timeout=5) # safe
requests.get("url", timeout=5.0) # safe
httpx.get("url", timeout=10) # safe

# --- CSP-D301/D302: Weak Hashes ---
hashlib.md5(b"abc") # unsafe (D301)
hashlib.sha1(b"abc") # unsafe (D302)
hashlib.new("md5", b"abc") # unsafe (D301)
hashlib.new("sha1", b"abc") # unsafe (D302)
hashlib.new("MD5", b"abc") # unsafe (D301)
hashlib.new("sha256", b"abc") # safe

# --- CSP-D304/D305: Ciphers and Modes ---
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
Cipher(algorithms.ARC4(key), modes.ECB()) # unsafe (ARC4: D304, ECB: D305)
Cipher(algorithms.Blowfish(key), modes.CBC(iv)) # unsafe (Blowfish: D304)
Cipher(algorithms.AES(key), modes.ECB()) # unsafe (ECB: D305)
Cipher(algorithms.AES(key), modes.GCM(iv)) # safe

# --- CSP-D311: Weak Randomness ---
random.random() # unsafe
random.randint(0, 10) # unsafe
random.choice([1, 2, 3]) # unsafe
import secrets
secrets.token_hex(16) # safe

# --- CSP-D104: XML ---
ET.parse("file.xml") # unsafe (D104, MEDIUM)
lxml.etree.parse("file.xml") # unsafe (D104, HIGH)
lxml.etree.fromstring("<root/>") # unsafe (D104, HIGH)
lxml.etree.RestrictedElement() # unsafe (D104, HIGH)

# --- CSP-D105: Assert ---
assert x == 1 # unsafe

# --- CSP-D106: Jinja2 Autoescape ---
Environment(autoescape=False) # unsafe
Environment(autoescape=True) # safe

# --- CSP-D504/D505/D506: Files/Temp ---
import tempfile
tempfile.mktemp() # unsafe (D504)
os.chmod("file", 0o777) # unsafe (D505 - world writable)
os.tempnam() # unsafe (D506)
os.tmpnam() # unsafe (D506)

# --- CSP-D403: Debug Mode ---
app.run(debug=True) # unsafe
app.run(debug=False) # safe

# --- CSP-D407/D408: Unverified SSL / HTTPS ---
ssl._create_unverified_context() # unsafe (D407)
import http.client
http.client.HTTPSConnection("host") # unsafe (D408 - missing context)
http.client.HTTPSConnection("host", context=ssl.create_default_context()) # safe

# --- CSP-D201/D203: Deserialization ---
pickle.loads(data) # unsafe (D201)
marshal.load(f) # unsafe (D203)
import pandas
pandas.read_pickle("file") # unsafe (D201)

# --- CSP-D402: SSRF ---
requests.get(url) # unsafe (dynamic positional)
requests.get("http://google.com") # safe (literal positional)
requests.get(url="http://google.com") # safe (literal keyword)
user_input = "http://evil.com"
requests.get(url=user_input) # unsafe (dynamic keyword - NEW)
httpx.get(url=user_input) # unsafe
requests.post(url=user_input) # unsafe
