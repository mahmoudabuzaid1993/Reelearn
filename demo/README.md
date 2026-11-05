# Demo video

`reelearn-demo.mp4` is a 3-minute narrated walkthrough built entirely from the running product:

| Part | Source |
|---|---|
| Company console (sign-in, dashboard, AI Reel Studio, approvals, reports) | The Next.js console driven by Playwright against the **live Rust API and worker** on this machine, seeded demo tenant, development AI providers |
| Employee and manager app | The Flutter app built for the web with its in-memory fake API (`--dart-define=API=fake`), driven through Flutter's semantics tree at phone size |
| Reel clips inside the app | Two animated stand-in clips (`scenes/reel.html`), because no AI video vendor key is configured on this machine |
| Title, problem, pipeline and closing cards | `scenes/card.html`, recorded as animations |
| Narration | macOS system voice (`say`), one file per scene |
| Edit | `build.py` with ffmpeg: cuts by recorder marks, time-fits each scene to `script.json`, frames at 1080p30, muxes narration and a subtitle track |

## Rebuild

```bash
# services: postgres, redis, object storage, API on :8080, worker, console on :3000
docker compose up -d postgres redis minio
(cd api && cargo run -p reelearn-api -- seed && cargo run -p reelearn-api & cargo run -p reelearn-worker &)
(cd web && NEXT_PUBLIC_API_MOCK=false pnpm dev -p 3000 &)

# the app for the web, with the demo clips next to it, served on :8099
W=/tmp/reelearn-demo && mkdir -p $W
node demo/record_clips.cjs $W/clips
(cd mobile && flutter build web --release --dart-define=API=fake --dart-define=UI_LOCALE=en \
   --dart-define=SAMPLE_VIDEO=http://localhost:8099/demo/reel1.mp4 \
   --dart-define=SAMPLE_VIDEO_2=http://localhost:8099/demo/reel2.mp4 \
 && mkdir -p build/web/demo && cp $W/clips/*.mp4 build/web/demo/ && cd build/web && python3 -m http.server 8099 &)

# narration (script.json), recordings, edit
python3 demo/narrate.py $W
node demo/record_phone.cjs $W/phone
node demo/record_cards.cjs $W/cards
node demo/record_web.cjs   $W/web 1
python3 demo/build.py $W demo/reelearn-demo.mp4
```

Change the story in `script.json` (text and seconds per scene; the seconds must add up to `total_seconds`), re-run `narrate.py` and `build.py`.
