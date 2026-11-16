#!/usr/bin/env python3
"""Two devices on one machine, through the command line, end to end.

    QURB=target/release/qurb python3 scripts/two_devices.py

Sets one device up, joins a second to it by its code (the numbers compared
and approved, as a person would), runs both, and checks what a person relies
on: a file put in one folder appears in the other, both ways; a file sent
from each arrives in the other's Downloads; every byte the same. Prints how
long each took.

Plain Python and the `qurb` program, nothing else, so it runs wherever qurb
does: written for the platform tests on Windows (.github/workflows/
platforms.yml), where the desktop window cannot be driven yet.
"""

import hashlib
import os
import re
import subprocess
import sys
import tempfile
import time

QURB = os.environ.get("QURB", "qurb")
WORK = tempfile.mkdtemp(prefix="qurb-two-")
LIMIT = float(os.environ.get("TWO_DEVICES_WAIT", "180"))


def qurb(*args, timeout=120):
    done = subprocess.run([QURB, *args], capture_output=True, text=True, timeout=timeout)
    assert done.returncode == 0, f"qurb {' '.join(args)} failed:\n{done.stdout}{done.stderr}"
    return done.stdout


def wait(what, check, limit=LIMIT):
    started = time.monotonic()
    while time.monotonic() - started < limit:
        found = check()
        if found:
            took = time.monotonic() - started
            print(f"  {what}: {took:.1f} s", flush=True)
            return found
        time.sleep(0.25)
    raise AssertionError(f"{what}: not within {limit:.0f} s")


def same_bytes(path, digest):
    try:
        with open(path, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest() == digest
    except OSError:
        return False


def status(folder):
    said = qurb("status", folder)
    identity = re.search(r"identity\s+(\S+)", said).group(1)
    downloads = re.search(r"sent here\s+saved to (.+)", said).group(1).strip()
    return identity, downloads


def pair(a, b):
    """`b` joins `a` by its code; `a` approves when the numbers agree."""
    host = subprocess.Popen([QURB, "pair", a], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, bufsize=1)
    said, code = "", None
    while code is None:
        line = host.stdout.readline()
        assert line, f"qurb pair showed no code:\n{said}"
        said += line
        found = re.search(r"(qurb1-\S+)", line)
        code = found.group(1) if found else None

    joining = subprocess.Popen([QURB, "join", b, code], stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True)
    while "Approve?" not in said:
        more = host.stdout.read(1)
        assert more, f"qurb pair ended without asking:\n{said}"
        said += more
    host.stdin.write("y\n")
    host.stdin.flush()
    rest, _ = host.communicate(timeout=120)
    assert "Paired with" in rest, f"qurb pair did not pair:\n{said}{rest}"
    joined, _ = joining.communicate(timeout=120)
    assert joining.returncode == 0, f"qurb join failed:\n{joined}"


def main():
    a, b = os.path.join(WORK, "a"), os.path.join(WORK, "b")
    print(f"qurb: {QURB}; working in {WORK}", flush=True)
    qurb("init", a)
    pair(a, b)
    print("paired", flush=True)
    a_id, a_downloads = status(a)
    b_id, b_downloads = status(b)

    logs = [open(os.path.join(WORK, f"{n}.log"), "w") for n in "ab"]
    running = [subprocess.Popen([QURB, "run", folder], stdout=log, stderr=subprocess.STDOUT)
               for folder, log in zip((a, b), logs)]
    failed = True
    try:
        # A folder in a folder, so separators are crossed too.
        os.makedirs(os.path.join(a, "notes", "2026"), exist_ok=True)
        with open(os.path.join(a, "notes", "2026", "from-a.txt"), "w") as f:
            f.write("written on a\n")
        wait("a file in a's folder, in b's",
             lambda: same_bytes(os.path.join(b, "notes", "2026", "from-a.txt"),
                                hashlib.sha256(b"written on a\n").hexdigest()))
        with open(os.path.join(b, "from-b.txt"), "w") as f:
            f.write("written on b\n")
        wait("a file in b's folder, in a's",
             lambda: same_bytes(os.path.join(a, "from-b.txt"), hashlib.sha256(b"written on b\n").hexdigest()))

        for name, sender, to, downloads in (("a-to-b.bin", a, b_id, b_downloads), ("b-to-a.bin", b, a_id, a_downloads)):
            data = os.urandom(3_200_000)
            loose = os.path.join(WORK, name)
            with open(loose, "wb") as f:
                f.write(data)
            digest = hashlib.sha256(data).hexdigest()
            qurb("send", sender, loose, "to", to)
            wait(f"{name} sent, in Downloads ({downloads})",
                 lambda: same_bytes(os.path.join(downloads, name), digest))
        failed = False
        print("all arrived, every byte the same", flush=True)
    finally:
        for process in running:
            process.terminate()
        for process in running:
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                process.kill()
        for log in logs:
            log.close()
        if failed:
            for n in "ab":
                print(f"--- {n}'s log, its end")
                with open(os.path.join(WORK, f"{n}.log"), errors="replace") as f:
                    print("".join(f.readlines()[-int(os.environ.get("TWO_DEVICES_LOG_LINES", "60")):]))
            for folder in (a, b):
                print(f"--- what {folder} holds")
                for top, _, names in os.walk(folder):
                    if ".qurb" not in top:
                        for name in names:
                            print(os.path.join(top, name))


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print(f"two devices: {e}", file=sys.stderr)
        sys.exit(1)
