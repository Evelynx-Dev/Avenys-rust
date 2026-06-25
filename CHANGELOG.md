# Mire / Avenys changelog

## 3.11.33 — 2025-06-25

### New PAL functions

#### WebSocket server PAL (`418f48f`)
Added server-side WebSocket PAL functions to `pal_ws.c`:
- `pal_ws_server_accept(fd)` — performs server-side WebSocket upgrade handshake
- `pal_ws_server_send_text(fd, data)` — sends unmasked text frame (server→client)
- `pal_ws_server_recv(fd, max)` — receives and unmasks client frame
- `pal_ws_server_close(fd)` — sends close frame and closes TCP connection

Implements RFC 6455 §5.2–5.3: frame encoding/decoding, XOR masking,
extended payload lengths (16-bit and 64-bit), base64 key encoding.
WSS (TLS) server stubs added; full TLS server support pending.

#### C object cache fix (`d35a9de`)
Fixed `.cobject_cache` path to use `runtime_base()` instead of compile-time
`CARGO_MANIFEST_DIR`. Ensures C object compilation works on installed binaries
(not just in development tree).

### Previous (3.11.33)
- String concatenation memory leak fix (#17)
- Dict overwrite/remove fixes
- PAL naming corrections
- O(1) hash table for managed tracking
- Clippy zero warnings
- 151 tests passing, 0 failures
