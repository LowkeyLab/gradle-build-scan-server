import gzip
import json

def decode_leb128_at(data, pos):
    result = 0
    shift = 0
    consumed = 0
    while pos + consumed < len(data):
        b = data[pos + consumed]
        consumed += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            break
        shift += 7
    return result, consumed

d = json.load(open("captured-output/payloads/20260221_004252.268-647fee21-bb6f-4519-a2e4-d26be9447b97.json"))
import base64
body = base64.b64decode(d["request"]["body"]["base64"])
idx = body.find(b'\x1f\x8b\x08')
data = gzip.decompress(body[idx:])

print("tacascer / omarchy context:")
start = 161560
print(data[start:start+40].hex())

print("Linux context:")
start = 40
print(data[start:start+100].hex())

print("compileKotlin context:")
start = 93980
print(data[start:start+60].hex())

