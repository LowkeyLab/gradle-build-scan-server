import sys


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 search_hex_context.py <file.bin> <string_to_search>")
        sys.exit(1)

    file_path = sys.argv[1]
    target_string = sys.argv[2]

    with open(file_path, "rb") as f:
        data = f.read()

    target_bytes = target_string.encode("utf-8")
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
        print(f"String '{target_string}' not found.")


if __name__ == "__main__":
    main()
