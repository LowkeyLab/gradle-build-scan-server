import json
import base64
import glob
import os
import gzip
import io

for filepath in sorted(glob.glob('/tmp/gradle-payloads/*.json')):
    print(f"--- {os.path.basename(filepath)} ---")
    with open(filepath, 'r') as f:
        data = json.load(f)
    
    req = data.get('request', {})
    uri = req.get('uri', '')
    method = req.get('method', '')
    print(f"Method: {method} URI: {uri}")
    
    body = req.get('body', {})
    if isinstance(body, dict) and 'base64' in body:
        b64 = body['base64']
        try:
            raw = base64.b64decode(b64)
            print(f"Body size: {len(raw)} bytes")
            
            # Let's check for gzip magic number (1f 8b)
            # Sometimes it's prepended with some metadata.
            # In the example: KMUAAgAWAAZHUkFETEUABTkuMy4xAAU0LjMuMh+LCAAAAAAAAP...
            # 28 C5 00 02 00 16 00 06 47 52 41 44 4c 45 00 05 39 2e 33 2e 31 00 05 34 2e 33 2e 32 1F 8B 08 00 ...
            # 47 52 41 44 4c 45 is 'GRADLE'
            # 39 2e 33 2e 31 is '9.3.1'
            # 34 2e 33 2e 32 is '4.3.2'
            
            gzip_start = raw.find(b'\x1f\x8b\x08')
            if gzip_start != -1:
                print(f"Found gzip stream at offset {gzip_start}")
                print(f"Metadata before gzip: {raw[:gzip_start]}")
                try:
                    gz_data = raw[gzip_start:]
                    uncompressed = gzip.decompress(gz_data)
                    print(f"Uncompressed size: {len(uncompressed)} bytes")
                    print(f"Uncompressed prefix: {uncompressed[:200]}")
                except Exception as e:
                    print(f"Gzip decompression failed: {e}")
            else:
                print(f"Raw body (first 100 bytes): {raw[:100]}")
        except Exception as e:
            print(f"Base64 decode failed: {e}")
    else:
        print(f"Body: {str(body)[:100]}...")
    print("\n")
