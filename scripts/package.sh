#!/usr/bin/env bash
# Writes ../<folder>-submission.zip: everything in this project except the
# target folder and STARTER.md. .git is kept so the commit history shows.
set -euo pipefail

cd "$(dirname "$0")/.."
name="$(basename "$PWD")"
zip_name="${name}-submission.zip"
cd ..
rm -f "$zip_name"

if command -v zip >/dev/null 2>&1; then
    zip -qr "$zip_name" "$name" -x "$name/target" "$name/target/*" "$name/STARTER.md"
else
    python3 - "$name" "$zip_name" <<'PY'
import os
import sys
import zipfile

src, out = sys.argv[1], sys.argv[2]
skip = os.path.join(src, "STARTER.md")
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
    for root, dirs, files in os.walk(src):
        if root == src:
            dirs[:] = [d for d in dirs if d != "target"]
        for file in files:
            path = os.path.join(root, file)
            if path != skip:
                archive.write(path)
PY
fi

echo "Wrote $(pwd)/$zip_name ($(du -h "$zip_name" | cut -f1))"
if [ ! -f "$name/design/architecture.png" ]; then
    echo "Note: design/architecture.png is missing. Export the diagram from Excalidraw first."
fi
