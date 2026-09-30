#!/usr/bin/env python3
"""MLLP receiver for tests: accepts COUNT messages (default 1) on
127.0.0.1:PORT, one connection each, answers each with an ACK carrying the
given code (AA, AE, AR) and exits. MSA-2 echoes the message's MSH-10, or
CONTROL_ID when one is given (to test an ACK for another message). The
number of messages received is written to stdout at the end.

    python3 tests/tools/mllp_ack_server.py 47201 AA &
    python3 tests/tools/mllp_ack_server.py 47201 AA 2 &
    python3 tests/tools/mllp_ack_server.py 47201 AA 1 OTHER-999 &
"""
import socket
import sys

port, code = int(sys.argv[1]), sys.argv[2]
count = int(sys.argv[3]) if len(sys.argv) > 3 else 1
fixed_id = sys.argv[4] if len(sys.argv) > 4 else None
srv = socket.socket()
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("127.0.0.1", port))
srv.listen(1)
srv.settimeout(30)
received = 0
for _ in range(count):
    try:
        conn, _ = srv.accept()
    except socket.timeout:
        break
    data = b""
    while not data.endswith(b"\x1c\r"):
        chunk = conn.recv(4096)
        if not chunk:
            break
        data += chunk
    msg = data.lstrip(b"\x0b").rstrip(b"\x1c\r").decode("utf-8", "replace")
    received += 1
    control_id = msg.split("\r")[0].split("|")[9] if msg.count("|") >= 9 else ""
    if fixed_id is not None:
        control_id = fixed_id
    ack = f"MSH|^~\\&|RCV||SND||20260101||ACK|ACK1|P|2.5\rMSA|{code}|{control_id}\r"
    conn.sendall(b"\x0b" + ack.encode() + b"\x1c\r")
    conn.close()
print(f"received={received}", flush=True)
