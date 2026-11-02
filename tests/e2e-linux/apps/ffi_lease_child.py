"""Two threads of one process act on one ref through the C ABI, so they contend for the process lease.

Arguments: the library path, the qualified ref, the holder's and the
contender's timeouts in milliseconds. The holder thread starts at once and
waits for the per-user lease the runner holds. The contender starts when a
line arrives on stdin and must time out on the process lease. Each thread
prints one JSON line with its result code and envelope as soon as it has it.
"""

import ctypes
import json
import sys
import threading

ACTION_SIZE = 96
HEADLESS = 0


def bind(path):
    lib = ctypes.CDLL(path)
    lib.ad_action_size.restype = ctypes.c_size_t
    lib.ad_adapter_create.restype = ctypes.c_void_p
    lib.ad_adapter_destroy.argtypes = [ctypes.c_void_p]
    lib.ad_execute_by_ref_timeout.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_int32,
                                              ctypes.c_int64, ctypes.POINTER(ctypes.c_void_p)]
    lib.ad_execute_by_ref_timeout.restype = ctypes.c_int32
    lib.ad_free_string.argtypes = [ctypes.c_void_p]
    lib.ad_last_error_message.restype = ctypes.c_char_p
    if lib.ad_action_size() != ACTION_SIZE:
        raise ValueError(f"the library's AdAction is {lib.ad_action_size()} bytes, not {ACTION_SIZE}")
    return lib


def act(lib, ref, timeout_ms, label, lock):
    adapter = lib.ad_adapter_create()
    action = ctypes.create_string_buffer(ACTION_SIZE)
    out = ctypes.c_void_p()
    try:
        code = lib.ad_execute_by_ref_timeout(adapter, ref.encode(), action, HEADLESS, timeout_ms, ctypes.byref(out))
        envelope = ctypes.string_at(out.value).decode() if out.value else None
        message = None if code == 0 else (lib.ad_last_error_message() or b"").decode()
    finally:
        if out.value:
            lib.ad_free_string(out)
        lib.ad_adapter_destroy(adapter)
    with lock:
        sys.stdout.write(json.dumps({"thread": label, "code": code, "envelope": envelope, "message": message}) + "\n")
        sys.stdout.flush()


def main(argv):
    if len(argv) != 4:
        raise ValueError("expected the library, the ref and two timeouts")
    lib = bind(argv[0])
    lock = threading.Lock()
    holder = threading.Thread(target=act, args=(lib, argv[1], int(argv[2]), "holder", lock))
    holder.start()
    sys.stdin.readline()
    contender = threading.Thread(target=act, args=(lib, argv[1], int(argv[3]), "contender", lock))
    contender.start()
    contender.join()
    holder.join()


if __name__ == "__main__":
    main(sys.argv[1:])
