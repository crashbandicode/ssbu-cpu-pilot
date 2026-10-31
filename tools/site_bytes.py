"""Print the 64 bytes at each main-text offset of a 13.0.5 main ELF as Rust arrays.

Usage: site_bytes.py <main-text.elf> <hex offset>...

Offsets are main-module virtual addresses (the same numbers the plugin adds
to the text base). The ELF used on 2026-10-31 was
/home/intpa/elite-rematch-work/re/main-text.elf. Read-only."""
import struct
import sys


def segments(data):
    if data[:4] != b'\x7fELF' or data[4] != 2:
        raise SystemExit('not an ELF64 file')
    phoff, = struct.unpack_from('<Q', data, 0x20)
    phentsize, phnum = struct.unpack_from('<HH', data, 0x36)
    for i in range(phnum):
        kind, _flags, offset, vaddr, _paddr, filesz = struct.unpack_from(
            '<IIQQQQ', data, phoff + i * phentsize)
        if kind == 1:  # PT_LOAD
            yield vaddr, offset, filesz


def sections(data):
    """Allocated sections with file contents, as (address, offset, size)."""
    shoff, = struct.unpack_from('<Q', data, 0x28)
    shentsize, shnum = struct.unpack_from('<HH', data, 0x3a)
    for i in range(shnum):
        _name, kind, _flags, addr, offset, size = struct.unpack_from(
            '<IIQQQQ', data, shoff + i * shentsize)
        if kind != 8 and size:  # not SHT_NOBITS
            yield addr, offset, size


def bytes_at(data, address, size=64):
    for vaddr, offset, filesz in list(segments(data)) + list(sections(data)):
        if vaddr <= address and address + size <= vaddr + filesz:
            start = offset + address - vaddr
            return data[start:start + size]
    raise SystemExit(f'{address:#x}: not in a loaded segment')


def rust_array(chunk):
    rows = [', '.join(f'0x{b:02x}' for b in chunk[i:i + 16]) for i in range(0, len(chunk), 16)]
    return '[\n        ' + ',\n        '.join(rows) + ',\n    ]'


def main(argv):
    if len(argv) < 3:
        raise SystemExit(__doc__)
    with open(argv[1], 'rb') as handle:
        data = handle.read()
    for text in argv[2:]:
        address = int(text, 16)
        print(f'// {address:#x}\n{rust_array(bytes_at(data, address))}')


if __name__ == '__main__':
    main(sys.argv)
