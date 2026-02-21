import gzip
import json

def decode_varint(data, pos):
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

def parse_string(data, pos, length_varint):
    str_len = length_varint >> 1
    is_ref = length_varint & 1
    if is_ref:
        return f"Ref({str_len})", pos
    else:
        s = data[pos:pos+str_len]
        try:
            return f"Str('{s.decode('utf-8')}')", pos + str_len
        except:
            return f"Bytes({s.hex()})", pos + str_len

print("Linux context:")
pos = 40
# We know the first bytes are a bit messed up without the global context. Let's just decode varints and strings.
while pos < 150:
    # try to read an event ID
    event_id, cons = decode_varint(data, pos)
    print(f"[{pos:3}] Event {event_id}", end="  |  ")
    pos += cons
    
    # Just show the next 5 varints to see what's what
    args = []
    temp_pos = pos
    for i in range(5):
        val, cons = decode_varint(data, temp_pos)
        args.append(val)
        temp_pos += cons
    print(f"Next varints: {args}")
    
    if event_id == 14:
        length_val, cons = decode_varint(data, pos)
        pos += cons
        s, pos = parse_string(data, pos, length_val)
        print(f"      -> AddDict: {s}")
    elif event_id == 17: # Just a guess
        # OS info?
        pass
    else:
        # Just advance by 1 to not get stuck if we are misaligned, 
        # or actually let's just dump the raw bytes
        pass

