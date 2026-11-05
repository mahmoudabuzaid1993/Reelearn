#!/usr/bin/env python3
"""Generate one narration file per scene of demo/script.json with the macOS system voice.

    python3 demo/narrate.py <workdir>

Writes <workdir>/audio/<scene>.wav and prints each narration length next to the scene length.
"""
import json
import pathlib
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parent
out = pathlib.Path(sys.argv[1]).resolve() / "audio"
out.mkdir(parents=True, exist_ok=True)
script = json.loads((root / "script.json").read_text())
assert sum(s["seconds"] for s in script["scenes"]) == script["total_seconds"], "scene seconds must add up to total_seconds"

for scene in script["scenes"]:
    aiff, wav = out / f"{scene['id']}.aiff", out / f"{scene['id']}.wav"
    subprocess.run(["say", "-v", script["voice"], "-r", str(script["rate"]), "-o", str(aiff), scene["text"]], check=True)
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(aiff), "-ar", "48000", "-ac", "2", str(wav)], check=True)
    aiff.unlink()
    d = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(wav)], capture_output=True, text=True).stdout)
    warn = "  <-- longer than its scene" if d + 0.6 > scene["seconds"] else ""
    print(f"{scene['id']:16s} narration {d:5.1f}s  scene {scene['seconds']:3d}s{warn}")
