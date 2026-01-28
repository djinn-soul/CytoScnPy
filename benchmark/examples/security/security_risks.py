"""
Security Risks Example
Run with: cytoscnpy examples/security_risks.py --danger --secrets
"""

import os
import subprocess
import pickle
import yaml
import hashlib
import requests

# CSP-D001: Eval
def unsafe_eval(user_input):
    eval(user_input)

# CSP-D002: Exec
def unsafe_exec(user_input):
    exec(user_input)

# CSP-D201: Pickle load
def unsafe_pickle(data):
    pickle.loads(data)

# CSP-D202: YAML load
def unsafe_yaml(data):
    yaml.load(data)

# CSP-D301: Weak hashing
def weak_hash(password):
    return hashlib.md5(password.encode()).hexdigest()

# CSP-D401: SSL verification disabled
def unsafe_request(url):
    requests.get(url, verify=False)

# CSP-D003: Command injection
def unsafe_subprocess(cmd):
    subprocess.run(cmd, shell=True)

# CSP-S001: Hardcoded secrets
AWS_KEY = "AKIAIOSFODNN7EXAMPLE"
STRIPE_KEY = "sk_live_51Mz..."
