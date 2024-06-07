# A manual script to package the contents of a directory into a zip file

import json
from pathlib import Path
from zipfile import ZipFile

ROOT_DIR = Path(__file__).resolve().parent.parent

PACKAGE_JSON = ROOT_DIR / "package.json"
DIST_FOLDER = ROOT_DIR / "dist"
BACKEND_FOLDER = ROOT_DIR / "backend"
DEFAULTS_FOLDER = ROOT_DIR / "defaults"

package_data = json.loads(PACKAGE_JSON.read_text())

name = package_data["name"]
version = package_data["version"]
output_file = ROOT_DIR / f"{name}-v{version}.zip"

with ZipFile(output_file, "w") as zipf:
    # Write root stuff
    zipf.write(PACKAGE_JSON, f"{name}/package.json")
    zipf.write(ROOT_DIR / "README.md", f"{name}/README.md")
    zipf.write(ROOT_DIR / "LICENSE", f"{name}/LICENSE")
    zipf.write(ROOT_DIR / "main.py", f"{name}/main.py")
    zipf.write(ROOT_DIR / "plugin.json", f"{name}/plugin.json")

    # Write binary
    zipf.write(BACKEND_FOLDER / "out" / "backend", f"{name}/bin/backend")

    # Write dist folder
    for file in DIST_FOLDER.rglob("*"):
        if file.is_file():
            zipf.write(file, f"{name}/dist/{file.relative_to(DIST_FOLDER)}")

    # Write defaults folder (to root)
    # defaults/file.txt into -> name/file.txt
    for file in DEFAULTS_FOLDER.rglob("*"):
        if file.is_file():
            zipf.write(file, f"{name}/{file.relative_to(DEFAULTS_FOLDER)}")
