import argparse
import json
import re
import struct
import subprocess
import zlib
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "940440952/venera-sources"
CDN = f"https://cdn.jsdelivr.net/gh/{REPOSITORY}@main"


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def icon_png():
    def chunk(kind, value):
        return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value))

    pixels = bytearray()
    for row in range(128):
        pixels.append(0)
        for column in range(128):
            is_book = 25 <= column < 103 and 24 <= row < 105
            is_line = 38 <= column < 91 and any(start <= row < start + 6 for start in (43, 61, 79))
            color = (18, 101, 112) if not is_book or is_line else (250, 246, 228)
            pixels.extend(color)
    header = struct.pack(">IIBBBBB", 128, 128, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(pixels))) + chunk(b"IEND", b"")


def add_entry(archive, name, data):
    entry = ZipInfo(name, (2026, 1, 1, 0, 0, 0))
    entry.compress_type = ZIP_DEFLATED
    entry.external_attr = 0o644 << 16
    archive.writestr(entry, data)


def run():
    arguments = argparse.ArgumentParser()
    arguments.add_argument("--catalogs-only", action="store_true")
    options = arguments.parse_args()
    venera_entries = []
    aidoku_entries = []
    for source_dir in sorted((ROOT / "sources").iterdir()):
        if not source_dir.is_dir():
            continue
        javascript = source_dir / "venera.js"
        if not javascript.is_file():
            raise RuntimeError(f"Missing Venera implementation: {source_dir.name}")
        source_code = javascript.read_text(encoding="utf-8")
        name = re.search(r'\bname\s*=\s*"([^"]+)"', source_code).group(1)
        version = re.search(r'\bversion\s*=\s*"([^"]+)"', source_code).group(1)
        relative_script = javascript.relative_to(ROOT).as_posix()
        venera_entries.append({"name": name, "version": version, "url": f"{CDN}/{relative_script}", "description": f"{source_dir.name}；关键词搜索可能需要网页验证，支持漫画数字 ID。"})
        crate = source_dir / "aidoku"
        manifest = json.loads((crate / "res/source.json").read_text(encoding="utf-8"))
        info = manifest["info"]
        filename = f"{info['id']}-v{info['version']}.aix"
        tag = f"aidoku-{info['id']}-v{info['version']}"
        aidoku_entries.append({
            "id": info["id"], "name": info["name"], "version": info["version"],
            "languages": info["languages"], "contentRating": info["contentRating"],
            "baseURL": info["url"], "minAppVersion": info["minAppVersion"],
            "downloadURL": f"https://github.com/{REPOSITORY}/releases/download/{tag}/{filename}",
        })
        if not options.catalogs_only:
            subprocess.run(["cargo", "build", "--manifest-path", str(crate / "Cargo.toml"), "--release", "--locked", "--target", "wasm32-unknown-unknown"], check=True)
            metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--manifest-path", str(crate / "Cargo.toml"), "--no-deps", "--format-version", "1"], text=True))
            wasm = Path(metadata["target_directory"]) / "wasm32-unknown-unknown/release" / f"{source_dir.name.replace('-', '_')}.wasm"
            if wasm.read_bytes()[:4] != b"\0asm":
                raise RuntimeError(f"Invalid wasm: {wasm}")
            output = ROOT / "dist" / filename
            output.parent.mkdir(parents=True, exist_ok=True)
            with ZipFile(output, "w") as archive:
                add_entry(archive, "Payload/main.wasm", wasm.read_bytes())
                add_entry(archive, "Payload/icon.png", icon_png())
                for resource in sorted((crate / "res").iterdir()):
                    if resource.is_file():
                        add_entry(archive, f"Payload/{resource.name}", resource.read_bytes())
            with ZipFile(output) as archive:
                if archive.testzip() is not None:
                    raise RuntimeError(f"Invalid package: {output}")
            print(output)
    write_json(ROOT / "index.json", venera_entries)
    write_json(ROOT / "aidoku/index.json", {"name": "漫画站双生态源库", "feedbackURL": f"https://github.com/{REPOSITORY}/issues", "sources": aidoku_entries})


if __name__ == "__main__":
    run()
