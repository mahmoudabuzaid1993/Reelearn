# Reelearn on Meta Model API — hackathon notes

Reelearn turns the documents, talks and podcasts a company already has into one-minute vertical learning reels for frontline staff, asks one recall question per reel, and re-asks it on a spaced schedule so the knowledge sticks. Saudi-first: Arabic and expat languages, shift-aware delivery, WhatsApp as a first-class channel, HRDF evidence.

**Track:** Startup (a real product with a real customer problem). **Theme:** Build for Your World — retail and warehouse teams in Saudi Arabia who never open an LMS.

## Where Meta Model API runs inside the product

| Muse model | What it does in Reelearn | Where |
|---|---|---|
| **Muse Spark 1.3** (`muse-spark-1.3`) | Reads the source (PDF, image, URL, topic brief), extracts key ideas, writes each reel's 45–60 s script and on-screen caption, writes the recall question with a warm explanation, translates script + question to Arabic/Urdu/Bengali, and finds teachable moments in podcast transcripts. Every call uses `response_format: json_schema`, so output is schema-constrained JSON and the pipeline never parses prose. | `api/crates/worker/src/providers/muse.rs` → `MuseWriter` |
| **Muse Image** (`muse-image-1.0`) | A 9:16 keyframe per reel from the script's moment and presenter (`1080x1920`, JPEG). ffmpeg turns the keyframe into a slow-zoom H.264 reel with the narration track; the keyframe is also the reel's poster in the app. | `MuseStillsRenderer` |
| **Muse Voice Transcribe** (`muse-voice-transcribe-1.0`) | Diarized transcription of uploaded town halls and podcasts (`POST /v1/asr/transcribe`, `mode: DIARIZATION`). Turns become the transcript Muse Spark mines for moments; ffmpeg cuts the chosen moment into a 9:16 clip. | `MuseTranscriber` |
| **WhatsApp Business Cloud API** | The same daily minute, question and feedback for learners who never install the app (bot replies, OTP, invites, nudges). | `providers/real.rs` → `MetaWhatsApp` |

What is *not* on Meta Model API, and what we did about it: there is no text-to-speech and no video-generation endpoint. Narration stays on a pluggable voice provider (fake in development), and video is keyframe + ffmpeg rather than a generated clip. Both are behind traits, so a Muse Video endpoint would be a one-file change.

## Setup

```bash
# 1. Infra and seed
docker compose up -d postgres redis minio
cd api && cp .env.example .env
cargo run -p reelearn-api -- seed

# 2. Meta Model API (key from dev.meta.ai → API keys)
export MODEL_API_KEY=...            # the only required variable
export PROVIDERS=real               # use the real adapters even in development
brew install ffmpeg                 # keyframe → video, WAV conversion for transcription

# 3. Run
cargo run -p reelearn-api           # http://localhost:8080  (Swagger at /docs)
cargo run -p reelearn-worker        # logs: "worker providers writer=muse-spark video=muse-image+ffmpeg asr=muse-voice-transcribe"
cd ../web && pnpm dev               # http://localhost:3000, sign in admin@alwadi.example, code 1234
```

Optional: `MUSE_MODEL=muse-spark-1.3-contributor` (cheaper tier), `MUSE_BASE_URL` (default `https://api.meta.ai/v1`), `WRITER_PROVIDER=claude` to A/B against Claude when `ANTHROPIC_API_KEY` is set.

## Demo script (3 minutes)

1. **Studio, S01 → S02:** upload the "Service standard" PDF, pick Customer service, 3 reels, Arabic + English, presenter Maha. The quote shows the credit cost before anything runs.
2. **S03:** watch the seven steps light up in order. Extract, script, question and translate are Muse Spark; render is Muse Image + ffmpeg; the step log names the provider.
3. **S04 storyboard:** three draft reels with scripts, keyframes, questions and Arabic variants. Open one in the editor, tweak a line.
4. **S07:** L&D approves, the SME approves, publish to "Store 12". The API refuses to publish before both approvals (database trigger).
5. **Phone:** Sara signs in with her number and the WhatsApp code, swipes through the reels, answers, gets mint feedback, never red. "Done for today" ends the stack. Rizwan gets the same minute on WhatsApp.
6. **Reports:** readiness by topic and team, the HRDF evidence CSV.

## Cost on the $150 hackathon credits

A 3-reel job in two languages makes about 8 Muse Spark calls (~25k input, ~6k output tokens ≈ $0.06), 3 Muse Image keyframes ($0.03) and, for a podcast source, transcription at $0.18/hour. A full demo run is well under a dollar; the credits cover thousands of reels.

## Proof it works without a key

`cargo test -p reelearn-worker --test muse` spins up a local mock of `api.meta.ai` (chat completions with JSON-schema answers, image generation, multipart transcription) and drives the real pipeline through it: schema names, bearer auth, `developer` role, `1080x1920` keyframes, WAV diarization requests and the stored posters are all asserted.
