#!/usr/bin/env python3
"""Check for Updates, rehearsed on the Mac or Linux before any newer
release exists (Windows has tools\\Rehearse-Update.ps1): a copy of a built
app in a scratch folder, a made-up "version 99.0.0" for it to find, and a
small web server on this computer only (127.0.0.1) that hands it out. The
copy runs on scratch data with one made-up location, so the real settings
are never touched.

    python3 rehearse-update.py Weatherspell.dmg               (the Mac)
    python3 rehearse-update.py Weatherspell-x86_64.AppImage   (Linux)

Options: --slow (the download takes about 20 seconds, so its progress is
heard and Cancel can be tried), --wrong-checksum (the release's checksum
does not match: Install refuses), --automatic ("Check for updates
automatically" starts turned on), --out FOLDER, --port N.

It stays running, serving, until the copy is closed (a relaunched copy
counts) or Ctrl+C. After Relaunch Now the new copy finds "99.0.0" again,
since it is the same build: that is the rehearsal, not a fault.

The address variable it sets is read only by previews and builds between
tags (window.rs, update_feed), such as CI's build of a branch.
"""

import argparse
import hashlib
import http.server
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time

parser = argparse.ArgumentParser(description="Rehearse Check for Updates.")
parser.add_argument("build", help="Weatherspell.dmg or Weatherspell-x86_64.AppImage")
parser.add_argument("--slow", action="store_true")
parser.add_argument("--wrong-checksum", action="store_true")
parser.add_argument("--automatic", action="store_true")
parser.add_argument("--out", default=os.path.join(tempfile.gettempdir(), "weatherspell-update-rehearsal"))
parser.add_argument("--port", type=int, default=8765)
args = parser.parse_args()

mac = sys.platform == "darwin"
name = "Weatherspell.dmg" if mac else "Weatherspell-x86_64.AppImage"
built = os.path.abspath(args.build)
if not os.path.isfile(built):
    sys.exit(f"{built} was not found.")
out = os.path.realpath(args.out)
serve, data, app_folder = (os.path.join(out, part) for part in ("serve", "data", "app"))
shutil.rmtree(app_folder, ignore_errors=True)
for folder in (serve, data, app_folder):
    os.makedirs(folder, exist_ok=True)

# The "new version". An AppImage runs the same with a few bytes after its
# end, so the swap shows as a changed file; a disk image is served as it
# is, since bytes after its end would spoil it.
new = os.path.join(serve, name)
shutil.copyfile(built, new)
if not mac:
    with open(new, "ab") as file:
        file.write(b"\nrehearsal copy\n")
with open(new, "rb") as file:
    checksum = hashlib.sha256(file.read()).hexdigest()
if args.wrong_checksum:
    checksum = "0" * 64
with open(os.path.join(serve, "SHA256SUMS"), "w", encoding="utf-8") as file:
    file.write(f"{checksum}  {name}\n")

base = f"http://127.0.0.1:{args.port}"
notes = (
    "A rehearsal of Check for Updates, not a real release.\r\n\r\n"
    "### Added\r\n"
    "- **Check for Updates** in the Help menu shows what's new and installs it (#11).\r\n"
    "- Settings can check for updates automatically.\r\n\r\n"
    "### Fixed\r\n"
    "- Nothing: these lines are here to be read with the arrow keys.\r\n"
)
releases = [
    {
        "tag_name": "v99.0.0",
        "draft": False,
        "prerelease": False,
        "html_url": "https://github.com/PlanetLinux98/weatherspell/releases",
        "body": notes,
        "assets": [
            {"name": name, "size": os.path.getsize(new), "browser_download_url": f"{base}/{name}"},
            {"name": "SHA256SUMS", "size": 83, "browser_download_url": f"{base}/SHA256SUMS"},
        ],
    }
]
with open(os.path.join(serve, "releases.json"), "w", encoding="utf-8") as file:
    json.dump(releases, file)

settings = {
    "version": 1,
    "locations": [
        {
            "name": "Peterborough",
            "region": "Ontario",
            "country": "Canada",
            "latitude": 44.30012,
            "longitude": -78.31623,
            "nickname": "Home",
        }
    ],
    "lastLocation": 0,
    "checkUpdatesAutomatically": args.automatic,
}
with open(os.path.join(data, "settings.json"), "w", encoding="utf-8") as file:
    json.dump(settings, file)

# The copy that updates itself.
if mac:
    volume = os.path.join(out, "volume")
    os.makedirs(volume, exist_ok=True)
    subprocess.run(
        ["hdiutil", "attach", "-quiet", "-nobrowse", "-readonly", "-mountpoint", volume, built],
        check=True,
    )
    try:
        app = os.path.join(app_folder, "Weatherspell.app")
        subprocess.run(["ditto", os.path.join(volume, "Weatherspell.app"), app], check=True)
    finally:
        subprocess.run(["hdiutil", "detach", "-quiet", volume], check=False)
else:
    app = os.path.join(app_folder, name)
    shutil.copyfile(built, app)
    os.chmod(app, 0o755)


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        wanted = self.path.lstrip("/")
        path = os.path.join(serve, wanted)
        if wanted not in ("releases.json", "SHA256SUMS", name) or not os.path.isfile(path):
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(os.path.getsize(path)))
        self.send_header("Connection", "close")
        self.end_headers()
        # Sixty or so pieces, each after a pause when --slow.
        piece = max(os.path.getsize(path) // 60, 64 * 1024)
        try:
            with open(path, "rb") as file:
                while block := file.read(piece):
                    self.wfile.write(block)
                    if args.slow and wanted == name:
                        time.sleep(0.3)
            print(f"  sent {wanted}", flush=True)
        except OSError:
            # The app's Cancel closes the connection mid-file.
            pass

    def log_message(self, *_):
        pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()

feed = f"{base}/releases.json"
if mac:
    # Launch Services passes no environment on unless told to.
    subprocess.run(
        ["open", "-n", app, "--env", f"WEATHERSPELL_DATA={data}", "--env", f"WEATHERSPELL_UPDATE_FEED={feed}"],
        check=True,
    )
else:
    subprocess.Popen(
        [app],
        env={**os.environ, "WEATHERSPELL_DATA": data, "WEATHERSPELL_UPDATE_FEED": feed},
        start_new_session=True,
    )
print(f"Weatherspell's rehearsal copy is running from {app_folder}.", flush=True)
print("Choose Help, Check for Updates. Close the copy to end the rehearsal.", flush=True)


# Programs started from the scratch folder: the folder begins the command
# (or follows a shell's name), so nothing that merely mentions it counts.
def copies():
    listed = subprocess.run(["ps", "-axo", "args="], capture_output=True, text=True).stdout
    return [line for line in listed.splitlines() if any(word.startswith(app_folder) for word in line.split()[:2])]


# A relaunch leaves a moment with no copy: five seconds without one is
# the end.
gone = 0
try:
    time.sleep(2)
    while gone < 20:
        time.sleep(0.25)
        gone = 0 if copies() else gone + 1
    print("The rehearsal copy has closed.")
except KeyboardInterrupt:
    pass
server.shutdown()
