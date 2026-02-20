import json
import base64
import glob
import gzip

filepath = sorted(glob.glob('/tmp/gradle-payloads/*.json'))[-1]
with open(filepath, 'r') as f:
    data = json.load(f)

req = data.get('request', {})
body = req.get('body', {})
raw = base64.b64decode(body['base64'])
gzip_start = raw.find(b'\x1f\x8b\x08')
gz_data = raw[gzip_start:]
uncompressed = gzip.decompress(gz_data)

print(uncompressed[:100].hex())
print(uncompressed[:100])
