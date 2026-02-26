import zlib
data = b'\x00\x00\x00\x00\x00\x09'
crc = zlib.crc32(data) & 0xffff
print(f"CRC of {data.hex()} is {hex(crc)}")
