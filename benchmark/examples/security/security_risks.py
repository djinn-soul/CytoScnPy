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

# SKY-D201: Eval
def unsafe_eval(user_input):
    eval(user_input)

# SKY-D202: Exec
def unsafe_exec(user_input):
    exec(user_input)

# SKY-D203: Pickle load
def unsafe_pickle(data):
    pickle.loads(data)

# SKY-D205: YAML load
def unsafe_yaml(data):
    yaml.load(data)

# SKY-D206: Weak hashing
def weak_hash(password):
    return hashlib.md5(password.encode()).hexdigest()

# SKY-D208: SSL verification disabled
def unsafe_request(url):
    requests.get(url, verify=False)

# SKY-D212: Command injection
def unsafe_subprocess(cmd):
    subprocess.run(cmd, shell=True)

# SKY-S101: Hardcoded secrets
AWS_KEY = "AKIAIOSFODNN7EXAMPLE"
STRIPE_KEY = "sk_live_51Mz..."
