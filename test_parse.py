import gzip
import json
import base64
import glob

filepath = sorted(glob.glob('/tmp/gradle-payloads/*.json'))[-1]
with open(filepath, 'r') as f:
    data = json.load(f)

req = data.get('request', {})
body = req.get('body', {})
raw = base64.b64decode(body['base64'])
gzip_start = raw.find(b'\x1f\x8b\x08')
gz_data = raw[gzip_start:]
uncompressed = gzip.decompress(gz_data)

class Decoder:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def read_varint(self):
        result = 0
        shift = 0
        while True:
            b = self.data[self.pos]
            self.pos += 1
            result |= (b & 0x7F) << shift
            if (b & 0x80) == 0:
                break
            shift += 7
        return result

    def read_string(self):
        val = self.read_varint()
        if val == 0:
            return None
        length = val >> 1
        is_ascii = (val & 1) == 0
        b = self.data[self.pos:self.pos+length]
        self.pos += length
        return b.decode('ascii' if is_ascii else 'utf-8')

d = Decoder(uncompressed)
for i in range(20):
    try:
        val = d.read_varint()
        # This will fail quickly if it's not varints, but let's see
        print(f"Varint {i}: {val} (hex: {hex(val)})")
    except:
        break

