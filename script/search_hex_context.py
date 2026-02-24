import sys


def encode_leb128(value):
    result = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value != 0:
            byte |= 0x80
        result.append(byte)
        if value == 0:
            break
    return bytes(result)


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 search_hex_context.py <file.bin> <target> [--varint]")
        sys.exit(1)

    file_path = sys.argv[1]
    target_raw = sys.argv[2]
    is_varint = "--varint" in sys.argv

    with open(file_path, "rb") as f:
        data = f.read()

    if is_varint:
        target_bytes = encode_leb128(int(target_raw))
        print(
            f"Searching for LEB128 encoded value: {' '.join(f'{b:02x}' for b in target_bytes)}"
        )
    else:
        target_bytes = target_raw.encode("utf-8")

    idx = 0
    matches = 0
    while True:
        idx = data.find(target_bytes, idx)
        if idx == -1:
            break

        print(f"Match found at offset {idx}")
        start = max(0, idx - 30)
        end = min(len(data), idx + len(target_bytes) + 50)
        chunk = data[start:end]

        hex_str = " ".join(f"{b:02x}" for b in chunk)
        print(f"HEX:  {hex_str}")

        idx += 1
        matches += 1

    if matches == 0:
        print(f"Target not found.")


if __name__ == "__main__":
    main()
