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
