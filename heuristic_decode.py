import gzip, json, base64, glob
import datetime

filepath = sorted(glob.glob('/tmp/gradle-payloads/*.json'))[-1]
with open(filepath, 'r') as f:
    data = json.load(f)

raw = base64.b64decode(data['request']['body']['base64'])
gzip_start = raw.find(b'\x1f\x8b\x08')
if gzip_start == -1:
    print("No gzip stream found")
    exit(1)
    
uncompressed = gzip.decompress(raw[gzip_start:])

class Decoder:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def read_varint(self):
        result = 0
        shift = 0
        while True:
            if self.pos >= len(self.data): raise EOFError()
            b = self.data[self.pos]
            self.pos += 1
            result |= (b & 0x7F) << shift
            if (b & 0x80) == 0:
                break
            shift += 7
        return result

d = Decoder(uncompressed)
parsed = []

while d.pos < len(d.data):
    start_pos = d.pos
    try:
        val = d.read_varint()
        
        # Check if it's a timestamp (between 2020 and 2030)
        if 1600000000000 < val < 1900000000000:
            dt = datetime.datetime.fromtimestamp(val / 1000.0, tz=datetime.timezone.utc)
            parsed.append(f"Timestamp: {dt.isoformat()}")
            continue
            
        # Check if it's an inline string
        length = val >> 1
        is_string = False
        if 2 <= length <= 500 and (val & 1) == 0:
            s_bytes = d.data[d.pos:d.pos+length]
            try:
                s = s_bytes.decode('utf-8')
                # Check if mostly printable (allow some newlines/tabs)
                if all(c.isprintable() or c in '\n\r\t' for c in s):
                    parsed.append(f"String: \"{s}\"")
                    d.pos += length
                    is_string = True
            except:
                pass
                
        if not is_string:
            if val < 10000:
                parsed.append(f"Int: {val}")
            else:
                parsed.append(f"Varint: {val} (hex: {hex(val)})")
                
    except EOFError:
        break
    except Exception as e:
        d.pos = start_pos + 1

with open('/tmp/decoded_trace.txt', 'w') as f:
    for p in parsed:
        f.write(p + "\n")

print(f"Decoded {len(parsed)} elements.")
