#!/usr/bin/env python3
import argparse
import base64
import glob
import gzip
import json


def load_payload(filepath=None):
    if not filepath:
        files = sorted(glob.glob("captured-output/payloads/*.json"))
        if not files:
            raise ValueError("No payload found in captured-output/payloads/")
        filepath = files[-1]

    with open(filepath, "r") as f:
        data = json.load(f)
    body = data.get("request", {}).get("body", {})
    raw = base64.b64decode(body["base64"])
    gzip_start = raw.find(b"\x1f\x8b\x08")
    return gzip.decompress(raw[gzip_start:])


class Decoder:
    def __init__(self, data):
        self.data = data
        self.pos = 0
        self.dict = []

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
        is_ref = (val & 1) == 1
        length_or_ref = val >> 1
        if is_ref:
            if length_or_ref < len(self.dict):
                return self.dict[length_or_ref]
            return f"INVALID_REF({length_or_ref})"
        else:
            s = self.data[self.pos : self.pos + length_or_ref]
            self.pos += length_or_ref
            return s.decode("utf-8")


def parse_payload(args):
    uncompressed = load_payload(args.file)
    d = Decoder(uncompressed)
    print(f"Header 1: {d.read_varint()}")
    print(f"Header 2: {d.read_varint()}")
    print(f"Header 3: {d.read_varint()}")

    while d.pos < len(d.data):
        start_pos = d.pos
        try:
            event = d.read_varint()
        except IndexError:
            break

        if event == 0:
            print(f"[{start_pos:4}] Event 0 (Timestamp): {d.read_varint()}")
        elif event == 14:
            s = d.read_string()
            d.dict.append(s)
            print(f"[{start_pos:4}] Event 14 (DictAdd): '{s}'")
        elif event == 517:
            print(f"[{start_pos:4}] Event 517 (BuildEnd? no args)")
        elif event == 9:
            print(f"[{start_pos:4}] Event 9: {d.read_varint()}, {d.read_varint()}")
        elif event == 1:
            print(f"[{start_pos:4}] Event 1: {d.read_varint()}")
        elif event == 8:
            print(f"[{start_pos:4}] Event 8 (OS)")
            print(f"      str1: '{d.read_string()}'")
            print(f"      str2: '{d.read_string()}'")
            print(f"      str3: '{d.read_string()}'")
        elif event == 2:
            print(
                f"[{start_pos:4}] Event 2 (Host): '{d.read_string()}', '{d.read_string()}'"
            )
        elif event == 1022:
            print(f"[{start_pos:4}] Event 1022: {d.read_varint()}, {d.read_varint()}")
        elif event == 3:
            print(f"[{start_pos:4}] Event 3 (JVM): {d.read_varint()}")
            for _ in range(6):
                print(f"      str: '{d.read_string()}'")
        elif event == 2109:
            print(f"[{start_pos:4}] Event 2109 (no args?)")
        elif event == 1007:
            print(f"[{start_pos:4}] Event 1007: {d.read_varint()}")
        elif event == 16:
            print(f"[{start_pos:4}] Event 16: {d.read_varint()}, {d.read_varint()}")
        elif event == 34:
            print(f"[{start_pos:4}] Event 34: {d.read_varint()}")
        elif event == 105:
            print(f"[{start_pos:4}] Event 105: {d.read_varint()}")
        else:
            print(f"[{start_pos:4}] UNKNOWN EVENT: {event}.")
            print("Next bytes:", d.data[d.pos : d.pos + 16].hex())
            break


def dump_payload(args):
    data = load_payload(args.file)
    start = args.offset
    end = start + args.length
    print(f"Dump from {start} to {end}:")
    print(data[start:end].hex(" "))


def main():
    parser = argparse.ArgumentParser(description="Gradle Build Scan Payload Analyzer")
    subparsers = parser.add_subparsers(dest="command", required=True)

    parse_parser = subparsers.add_parser("parse", help="Parse the LEB128 binary stream")
    parse_parser.add_argument(
        "--file", help="Specific payload file to parse (defaults to latest)"
    )
    parse_parser.set_defaults(func=parse_payload)

    dump_parser = subparsers.add_parser("dump", help="Hex dump the payload")
    dump_parser.add_argument("--file", help="Specific payload file")
    dump_parser.add_argument("--offset", type=int, default=0, help="Start offset")
    dump_parser.add_argument(
        "--length", type=int, default=64, help="Number of bytes to dump"
    )
    dump_parser.set_defaults(func=dump_payload)

    args = parser.parse_args()

    if hasattr(args, "func"):
        args.func(args)


if __name__ == "__main__":
    main()
