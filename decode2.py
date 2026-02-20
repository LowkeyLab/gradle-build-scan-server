import json
import base64
import glob
import os
import gzip

for filepath in sorted(glob.glob('/tmp/gradle-payloads/*.json')):
    with open(filepath, 'r') as f:
        data = json.load(f)
    
    req = data.get('request', {})
    uri = req.get('uri', '')
    
    if uri.endswith('/token'):
        body = req.get('body', {})
        if isinstance(body, dict) and 'base64' in body:
            raw = base64.b64decode(body['base64'])
            print("Token Request Body:", raw)
        else:
            print("Token Request Body:", body)
