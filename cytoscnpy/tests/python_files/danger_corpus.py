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
pathlib.Path("safe/path")  # Safe
Path(user_input)  # Unsafe (if imported as Path)
zipfile.Path("archive.zip", at=user_input)  # Unsafe (dynamic path inside zip)
