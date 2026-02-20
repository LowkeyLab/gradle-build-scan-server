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
while d.pos < 300:
    start_pos = d.pos
    try:
        val = d.read_varint()
        # Assume it's a string if val >> 1 is between 1 and 100
        length = val >> 1
        if 2 <= length <= 60 and val & 1 == 0:
            s = d.data[d.pos:d.pos+length]
            try:
                dec = s.decode('ascii')
                # only printable
                if dec.isprintable():
                    print(f"[{start_pos:04x}] String? Length {length} -> '{dec}'")
                    d.pos += length
                    continue
            except:
                pass
        print(f"[{start_pos:04x}] Varint: {val}")
    except Exception as e:
        print(e)
        break
