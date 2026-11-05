#!/usr/bin/env python3
"""Assemble the Reelearn demo video from the recordings.

    python3 demo/build.py <workdir> [output.mp4]

<workdir> holds what the recorders wrote: web/web.mp4, phone/phone.mp4, phone/manager.mp4,
cards/card_*.mp4, cards/bg_*.png, cards/lower_*.png, audio/<scene>.wav. Every scene is cut from
its recording by the recorder's marks, time-fitted to the scene length in script.json, framed at
1920x1080/30 fps, then the scenes are joined, the narration is laid under them and an SRT is written.
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
WORK = pathlib.Path(sys.argv[1]).resolve()
OUT = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else ROOT / "reelearn-demo.mp4").resolve()
PARTS = WORK / "parts"
PARTS.mkdir(parents=True, exist_ok=True)
FPS = 30
W, H = 1920, 1080
# Where the phone recording sits inside bg_*.png (see scenes/frame.html).
PHONE = dict(x=1268, y=60, w=444, h=961)

script = json.loads((ROOT / "script.json").read_text())


def run(args):
    r = subprocess.run(args, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit("ffmpeg failed:\n" + " ".join(map(str, args[:12])) + " ...\n" + r.stderr[-1500:])


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)], capture_output=True, text=True).stdout
    return float(out.strip() or 0)


def marks(path):
    return {m["name"]: m["t"] for m in json.loads(pathlib.Path(path).read_text())}


def last_mark(path, prefix):
    return [m["t"] for m in json.loads(pathlib.Path(path).read_text()) if m["name"].startswith(prefix)]


def cut(src, start, end, target, out, kind, lower=None, bg=None):
    """One part: [start, end] of src, speed-fitted to `target` seconds, framed, written to `out`."""
    span = max(end - start, 0.1)
    factor = target / span  # > 1 slows the footage down
    chain = f"[0:v]trim=start={start:.3f}:end={end:.3f},setpts=(PTS-STARTPTS)*{factor:.5f},fps={FPS}"
    inputs = ["-i", str(src)]
    if kind == "web":
        chain += f",scale={W}:{H}:flags=lanczos,format=yuv420p[v0]"
        last = "v0"
    elif kind == "card":
        chain += f",scale={W}:{H}:flags=lanczos,format=yuv420p[v0]"
        last = "v0"
    else:  # phone: recording scaled into the bezel hole of the background still
        chain += f",scale={PHONE['w']}:{PHONE['h']}:flags=lanczos[ph]"
        inputs += ["-loop", "1", "-t", f"{target:.3f}", "-i", str(bg)]
        chain += f";[1:v]fps={FPS},format=yuv420p[bg];[bg][ph]overlay={PHONE['x']}:{PHONE['y']}:shortest=0[v0]"
        last = "v0"
    if lower:
        idx = inputs.count("-i")
        inputs += ["-loop", "1", "-t", f"{target:.3f}", "-i", str(lower)]
        show_until = min(5.2, target - 0.6)
        chain += f";[{idx}:v]format=rgba,fade=t=in:st=0.4:d=0.4:alpha=1,fade=t=out:st={show_until:.2f}:d=0.4:alpha=1[lt];[{last}][lt]overlay=0:0[v1]"
        last = "v1"
    chain += f";[{last}]trim=duration={target:.3f},setpts=PTS-STARTPTS,format=yuv420p[v]"
    run(["ffmpeg", "-y", "-loglevel", "error", *inputs, "-filter_complex", chain, "-map", "[v]", "-r", str(FPS), "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p", "-an", str(out)])
    return factor


web = WORK / "web" / "web.mp4"
wm = marks(WORK / "web" / "web.marks.json")
phone = WORK / "phone" / "phone.mp4"
pm = marks(WORK / "phone" / "phone.marks.json")
manager = WORK / "phone" / "manager.mp4"
mm = marks(WORK / "phone" / "manager.marks.json")
cards = WORK / "cards"
signins = last_mark(WORK / "web" / "web.marks.json", "signed in as")


def plan(scene):
    """The footage pieces of a scene as (src, start, end, share-of-scene, kind, extra)."""
    sid, secs = scene["id"], scene["seconds"]
    lower = cards / f"lower_{sid}.png"
    if scene["kind"] == "card":
        return [(cards / f"card_{sid}.mp4", 0.35, 0.35 + secs, 1.0, "card", {})]
    if sid == "03_console":
        return [(web, 0.4, wm["studio_new"] - 0.25, 1.0, "web", {"lower": lower})]
    if sid == "04a_studio":
        return [(web, wm["studio_new"], wm["generating"] + 1.6, 1.0, "web", {"lower": lower})]
    if sid == "04c_storyboard":
        return [(web, wm["storyboard"] + 0.3, wm["saved"] + 0.6, 1.0, "web", {"lower": lower})]
    if sid == "05_approval":
        return [
            (web, wm["publish_blocked"], wm["ld_approved"] + 0.5, 0.42, "web", {"lower": lower}),
            (web, wm["sme_approved"] - 4.4, wm["sme_approved"] - 0.1, 0.24, "web", {}),
            (web, wm["published"] - 4.6, wm["published"] + 3.0, 0.34, "web", {}),
        ]
    if sid == "06_phone":
        return [(phone, pm["start"] - 0.4, pm["end"], 1.0, "phone", {"bg": cards / f"bg_{sid}.png"})]
    if sid == "07_manager":
        return [
            (manager, mm["team"] - 0.6, mm["end"], 0.52, "phone", {"bg": cards / f"bg_{sid}.png"}),
            (web, wm["reports"] + 0.4, wm["end"], 0.48, "web", {"lower": cards / f"lower_{sid}.png"}),
        ]
    raise SystemExit(f"no plan for {sid}")


part_files = []
timeline = []
t = 0.0
for scene in script["scenes"]:
    pieces = plan(scene)
    for n, (src, start, end, share, kind, extra) in enumerate(pieces):
        target = round(scene["seconds"] * share, 3)
        out = PARTS / f"{scene['id']}_{n}.mp4"
        factor = cut(src, start, end, target, out, kind, **extra)
        note = "" if 0.6 <= factor <= 1.7 else "   <-- large speed change"
        print(f"{scene['id']:16s} part {n}: {end - start:6.1f}s of footage -> {target:5.1f}s (x{1 / factor:.2f} speed){note}")
        part_files.append(out)
    timeline.append((scene, t))
    t += scene["seconds"]

# Join the parts.
concat = PARTS / "concat.txt"
concat.write_text("".join(f"file '{p}'\n" for p in part_files))
video = PARTS / "video.mp4"
run(["ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", str(concat), "-c", "copy", str(video)])

# Narration: each scene's audio delayed to its start (plus a short lead-in), mixed on a silent bed.
LEAD = 0.45
inputs, filt, labels = ["-f", "lavfi", "-t", f"{t:.3f}", "-i", "anullsrc=r=48000:cl=stereo"], [], ["[0:a]"]
for i, (scene, start) in enumerate(timeline, start=1):
    inputs += ["-i", str(WORK / "audio" / f"{scene['id']}.wav")]
    ms = int((start + LEAD) * 1000)
    filt.append(f"[{i}:a]adelay={ms}|{ms},volume=1.0[a{i}]")
    labels.append(f"[a{i}]")
filt.append("".join(labels) + f"amix=inputs={len(labels)}:normalize=0:duration=first,alimiter=limit=0.95[a]")
audio = PARTS / "audio.m4a"
run(["ffmpeg", "-y", "-loglevel", "error", *inputs, "-filter_complex", ";".join(filt), "-map", "[a]", "-c:a", "aac", "-b:a", "192k", str(audio)])

# Subtitles: sentences spread across each scene's narration by length.
def stamp(x):
    ms = int(round(x * 1000))
    return f"{ms // 3600000:02d}:{ms // 60000 % 60:02d}:{ms // 1000 % 60:02d},{ms % 1000:03d}"


def sentences(text):
    out, cur = [], ""
    for ch in text:
        cur += ch
        if ch in ".:?!" and len(cur.strip()) > 18:
            out.append(cur.strip())
            cur = ""
    if cur.strip():
        out.append(cur.strip())
    return out


srt, n = [], 1
for scene, start in timeline:
    dur = duration(WORK / "audio" / f"{scene['id']}.wav")
    parts = sentences(scene["text"].replace("A P I", "API").replace("A I ", "AI ").replace("Next J S", "Next.js").replace("H R ", "HR "))
    total = sum(len(p) for p in parts) or 1
    cursor = start + LEAD
    for p in parts:
        d = dur * len(p) / total
        srt.append(f"{n}\n{stamp(cursor)} --> {stamp(cursor + d - 0.05)}\n{p}\n")
        cursor += d
        n += 1
srt_path = OUT.with_suffix(".srt")
srt_path.write_text("\n".join(srt))

# Mux: video + narration + a soft subtitle track players can switch on.
run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(video), "-i", str(audio), "-i", str(srt_path), "-map", "0:v", "-map", "1:a", "-map", "2:s", "-c:v", "copy", "-c:a", "copy", "-c:s", "mov_text", "-metadata:s:s:0", "language=eng", "-movflags", "+faststart", "-t", f"{t:.3f}", str(OUT)])
print(f"\nwrote {OUT}  ({duration(OUT):.2f}s, {OUT.stat().st_size / 1e6:.1f} MB)\nwrote {srt_path}")
