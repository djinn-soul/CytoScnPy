import os
import sys
import pickle
import yaml
import hashlib
import requests
import subprocess
import tarfile
import zipfile
import xml.etree.ElementTree as ET
from xml.dom import minidom
import xml.sax
import lxml.etree
import dill
import shelve
import xmlrpc
import xmlrpc.client
from Crypto.PublicKey import RSA
from wsgiref.handlers import CGIHandler

# CSP-D001: Eval
eval("1 + 1")

# CSP-D002: Exec
exec("print('hello')")

# CSP-D201: Pickle
pickle.load(open("data.pkl", "rb"))
pickle.loads(b"cos\nsystem\n(S'ls'\ntR.")

# CSP-D202: Yaml
yaml.load("!!python/object/apply:os.system [ls]", Loader=yaml.Loader)
yaml.load(stream) # Unsafe default

# CSP-D301: Weak Hash
hashlib.md5(b"hello")
hashlib.sha1(b"hello")

# CSP-D401: HTTP Verify
requests.get("https://example.com", verify=False)

# CSP-D003: Subprocess
os.system("ls " + sys.argv[1]) # Injection
subprocess.call("ls " + sys.argv[1], shell=True)
subprocess.run(args="ls " + sys.argv[1], shell=True)

# CSP-D101: SQL Injection (ORM)
cursor.execute(f"SELECT * FROM users WHERE name = '{name}'")
cursor.execute("SELECT * FROM users WHERE name = '{}'".format(name))
cursor.execute("SELECT * FROM users WHERE name = '" + name + "'")
cursor.execute("SELECT * FROM users WHERE name = '%s'" % name)

# CSP-D501: Path Traversal
open(sys.argv[1])
os.path.join(root, user_input)
shutil.copy(sys.argv[1], "dest")

# CSP-D402: SSRF
requests.get(user_url)
httpx.post(user_url)
urllib.request.urlopen(user_url)

# CSP-D102: Raw SQL
sqlalchemy.text(f"SELECT * FROM {table}")
pandas.read_sql(f"SELECT * FROM {table}", con)
User.objects.raw(f"SELECT * FROM users WHERE name = '{name}'")

# CSP-D103: XSS
flask.render_template_string(user_template)
jinja2.Markup(user_content)

# CSP-D104: XML
ET.parse("evil.xml")
ET.fromstring(xml_str)
ET.XML(xml_str)
xml.dom.minidom.parse("evil.xml")
xml.sax.parse("evil.xml")
lxml.etree.parse("evil.xml")

# CSP-D502: Tarfile
t = tarfile.open("archive.tar")
t.extractall() # Unsafe
t.extractall(filter=unsafe_filter) # Unsafe filter
t.extractall(filter='data') # Safe

# CSP-D503: Zipfile
z = zipfile.ZipFile("archive.zip")
z.extractall()

# Helper coverage (is_likely_tarfile_receiver)
tf = tarfile.TarFile("archive.tar")
tf.extractall()
self.tar.extractall()

# ════════════════════════════════════════════════════════════════════════
# Category 9: Modern Python Patterns (CSP-D9xx) - 2025/2026 Security
# ════════════════════════════════════════════════════════════════════════

# CSP-D901: Async subprocess security
import asyncio
asyncio.create_subprocess_shell(user_cmd)  # Unsafe - dynamic
asyncio.create_subprocess_shell("ls -la")  # Safe - static

os.popen(user_cmd)  # Unsafe
os.popen("ls")  # Safe

import pty
pty.spawn(user_shell)  # Unsafe

# CSP-D902: ML model deserialization
import torch
torch.load("model.pt")  # Unsafe - no weights_only
torch.load("model.pt", weights_only=True)  # Safe
torch.load("model.pt", weights_only=False)  # Unsafe

import joblib
joblib.load("model.pkl")  # Unsafe - always risky

from keras.models import load_model
load_model("model.h5")  # Unsafe - no safe_mode
load_model("model.h5", safe_mode=True)  # Safe
keras.models.load_model("model.h5")  # Unsafe
keras.load_model("model.h5")  # Unsafe - Added for CSP-D902
keras.load_model("trusted_model.h5", safe_mode=True) # Safe - Added negative case

# CSP-D903: Sensitive data in logs
import logging
password = "secret123"
token = "abc123"
api_key = "key123"

logging.debug(f"User password: {password}")  # Unsafe
logging.info("Processing token: " + token)  # Unsafe
logger.warning(api_key)  # Unsafe
logging.info("User logged in")  # Safe

# ════════════════════════════════════════════════════════════════════════
# New Security Gap Closures (2026-01-17)
# ════════════════════════════════════════════════════════════════════════

# CSP-D409: ssl.wrap_socket (deprecated and often insecure)
import ssl
ssl.wrap_socket(sock)  # Unsafe

# CSP-D004: wsgiref imports (httpoxy vulnerability)
import wsgiref  # Low severity audit
from wsgiref.handlers import CGIHandler  # High severity (already in imports above)

# CSP-D004: xmlrpclib (Python 2 legacy)
import xmlrpclib  # Unsafe - Python 2 XML-RPC

# CSP-D504: mktemp direct import
from tempfile import mktemp
mktemp()  # Unsafe - race condition

# CSP-D904: Django SECRET_KEY hardcoding
# CSP-D501: Modern Path Traversal (pathlib / zipfile)
import pathlib
import zipfile
pathlib.Path(user_input)  # Unsafe
pathlib.Path("safe/path")  # Safe - Negative case
from pathlib import Path, PurePath, PosixPath, WindowsPath
Path(user_input)  # Unsafe (if imported as Path)
PurePath(user_input) # Unsafe
PosixPath(user_input) # Unsafe
WindowsPath(user_input) # Unsafe
zipfile.Path("archive.zip", at=user_input)  # Unsafe (dynamic path inside zip)
zipfile.Path("archive.zip", path=user_input) # Unsafe (keyword 'path')
zipfile.Path("archive.zip", filename=user_input) # Unsafe (keyword 'filename')
zipfile.Path("archive.zip", filepath=user_input) # Unsafe (keyword 'filepath')
tarfile.TarFile("archive.tar").extractall(member=user_input) # Unsafe (keyword 'member')
zipfile.Path(user_input) # Unsafe (positional)
# Negative cases (literals)
Path("/etc/passwd") # Safe (literal)
PurePath("C:\\Windows") # Safe (literal)
zipfile.Path("archive.zip", at="data/file.txt") # Safe (literal)
# Multi-argument path traversal (Comment 1)
pathlib.Path("safe_prefix", user_input) # Unsafe (dynamic second arg)
os.path.join("safe", user_input) # Unsafe
os.path.abspath(user_input) # Unsafe (Comment 1)
# Multi-line cases for SSRF (Comment 1)
requests.get(
    url=user_input
)

# Expand SQLi/XSS# Template and JinjaSQL (Comment 1)
from string import Template
user_sql = input()
user_params = {"id": input()}
Template(user_sql).substitute(user_params) # Unsafe
Template("$sql").substitute(sql=user_sql) # Unsafe

from jinjasql import JinjaSql
j = JinjaSql()
query, params = j.prepare_query(user_sql, user_params) # Unsafe
j.prepare_query("SELECT * FROM table WHERE id={{id}}", user_params) # Unsafe (params dynamic)

import flask
flask.Markup(user_html)  # CSP-D103
from django.utils.html import format_html
format_html("<b>{}</b>", user_html)  # CSP-D103
from fastapi import HTMLResponse
HTMLResponse(content=user_html)  # CSP-D103

# Refined Literal Argument Checking (Regression Tests)
import requests
import os
import subprocess
t = 10
d = {"key": "value"}
requests.get("https://safe.com", timeout=t) # Safe: URL is literal
requests.post("https://safe.com", data=d) # Safe: URL is literal
os.system("ls") # Safe: Literal command
subprocess.run(["ls"], shell=True, timeout=t) # Safe: Command is literal list

# Further Security Rule Refinements (Regression Tests)
import asyncio
from fastapi import HTMLResponse
import os

# Comment 1: Path Traversal focuses on index 0
open("literal.txt", mode=os.environ.get("MODE", "r")) # Safe
asyncio.run(asyncio.create_subprocess_shell("ls", stdout=asyncio.PIPE)) # Safe

# Comment 2: XSS restricts keywords
HTMLResponse(content="<b>Safe</b>", status_code=os.getpid()) # Safe
HTMLResponse(content=os.environ.get("HTML"), status_code=200) # Unsafe (content is dynamic)

# Comment 3: os.path track all positional args (Taint analysis)
# This is better verified in dedicated taint tests, but we'll add the pattern here.
os.path.join("a", "b", os.environ.get("TAINTED")) # Should be flagged in taint mode
