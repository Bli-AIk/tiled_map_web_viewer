#!/usr/bin/env python3
"""Generate assets/manifest.txt listing all .tmx files under assets/."""
import os
import sys

assets_dir = "assets"
maps = []

for root, _dirs, files in os.walk(assets_dir):
    for f in sorted(files):
        if f.lower().endswith(".tmx"):
            rel = os.path.relpath(os.path.join(root, f), assets_dir)
            maps.append(rel)

maps.sort()

manifest_path = os.path.join(assets_dir, "manifest.txt")
with open(manifest_path, "w") as out:
    for m in maps:
        out.write(m + "\n")

print(f"Generated {manifest_path} with {len(maps)} map(s)", file=sys.stderr)
