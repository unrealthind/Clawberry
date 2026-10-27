#!/usr/bin/env python3
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import zipfile

parser = argparse.ArgumentParser(description="Build a networkless offline-wheel acceptance image")
parser.add_argument("base_image")
args = parser.parse_args()
if not re.fullmatch(r"[^\s]+@sha256:[a-f0-9]{64}", args.base_image) or args.base_image.startswith("-"):
    parser.error("A reviewed local image digest is required")

image = "localhost/raaviclaw-sandbox:package-acceptance"
distribution = "raaviclaw_acceptance_fixture"
metadata = f"{distribution}-0.0.1.dist-info"
files = {
    f"{distribution}/__init__.py": b'MARKER = "offline-fixture-installed"\n',
    f"{metadata}/METADATA": b"Metadata-Version: 2.1\nName: raaviclaw-acceptance-fixture\nVersion: 0.0.1\n",
    f"{metadata}/WHEEL": b"Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
}
records = []
for name, content in files.items():
    digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=").decode()
    records.append(f"{name},sha256={digest},{len(content)}")
records.append(f"{metadata}/RECORD,,")
files[f"{metadata}/RECORD"] = ("\n".join(records) + "\n").encode()

with tempfile.TemporaryDirectory(prefix="raaviclaw-wheel-fixture-") as directory:
    root = Path(directory)
    wheel = root / f"{distribution}-0.0.1-py3-none-any.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        for name, content in files.items():
            archive.writestr(zipfile.ZipInfo(name), content)
    containerfile = root / "Containerfile"
    containerfile.write_text(
        f"FROM {args.base_image}\nUSER 0:0\nCOPY {wheel.name} /opt/wheels/\nUSER 1000:1000\n"
    )
    subprocess.run(
        ["podman", "build", "--pull=never", "--network=none", "-t", image, "-f", str(containerfile), str(root)],
        check=True,
        stdout=sys.stderr,
    )
references = json.loads(subprocess.check_output(
    ["podman", "image", "inspect", image, "--format", "{{json .RepoDigests}}"], text=True
))
reference = next(value for value in references if value.startswith("localhost/raaviclaw-sandbox@sha256:"))
print(reference)
