from flask import request
import os

def vulnerable_func():
    cmd = request.args.get('cmd')
    # Finding on next line should be present (standard + taint)
    os.system(cmd) 

def suppressed_generic():
    cmd = request.args.get('cmd')
    # Finding on next line should be suppressed by generic noqa
    os.system(cmd) # noqa: CSP

def suppressed_specific():
    cmd = request.args.get('cmd')
    # Finding on next line should be suppressed by specific rule ID
    # CSP-D003 is for Command Injection
    os.system(cmd) # noqa: CSP-D003

def suppressed_mismatch():
    cmd = request.args.get('cmd')
    # Finding on next line should NOT be suppressed (wrong code)
    os.system(cmd) # noqa: CSP-X999
