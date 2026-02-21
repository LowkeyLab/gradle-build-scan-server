import gzip, json, base64, glob
filepath = sorted(glob.glob('/tmp/gradle-payloads/*.json'))[-1]
with open(filepath, 'r') as f:
    data = json.load(f)
raw = base64.b64decode(data['request']['body']['base64'])
uncompressed = gzip.decompress(raw[raw.find(b'\x1f\x8b\x08'):])

class Decoder:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def read_varint(self):
        result = 0
        shift = 0
        while True:
            if self.pos >= len(self.data): raise Exception("EOF")
            b = self.data[self.pos]
            self.pos += 1
            result |= (b & 0x7F) << shift
            if (b & 0x80) == 0:
                break
            shift += 7
        return result

d = Decoder(uncompressed)
for _ in range(10):
    start = d.pos
    val1 = d.read_varint()
    val2 = d.read_varint()
    print(f"Event/Type: {val1}, Length/Value: {val2}")
