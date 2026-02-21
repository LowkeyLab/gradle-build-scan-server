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
            b = self.data[self.pos]
            self.pos += 1
            result |= (b & 0x7F) << shift
            if (b & 0x80) == 0: break
            shift += 7
        return result

d = Decoder(uncompressed)
while d.pos < 500:
    try:
        val = d.read_varint()
        # string check
        length = val >> 1
        bit = val & 1
        if 2 <= length <= 100 and bit == 0:
            s_bytes = d.data[d.pos:d.pos+length]
            try:
                s = s_bytes.decode('utf-8')
                if s.isprintable():
                    print(f"String: {s}")
                    d.pos += length
                    continue
            except: pass
        if length > 0 and bit == 1:
            # maybe string ref?
            print(f"StringRef: {length}")
    except: break
