import ctypes, ctypes.util, platform
libc = ctypes.CDLL(ctypes.util.find_library("c") or None)
libc.malloc.restype = ctypes.c_void_p
libc.malloc.argtypes = [ctypes.c_size_t]
for label, size in [("2^60 bits -> 2^58 bytes", 1 << 58), ("u64::MAX bits -> 2^62 bytes", 1 << 62)]:
    p = libc.malloc(size)
    print(platform.system(), platform.machine(), label, "->", "NULL (refused)" if not p else "SUCCEEDED %#x" % p, flush=True)
