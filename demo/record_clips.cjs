// Records the two stand-in reel clips (animated HTML scenes) as vertical MP4s.
//   node demo/record_clips.cjs <outDir>
const path = require("path");
const { execFileSync } = require("child_process");
const { launch, hold } = require("./lib.cjs");
(async () => {
  const outDir = process.argv[2];
  for (const v of ["1", "2"]) {
    const { page, finish } = await launch({ width: 540, height: 960, scale: 2, outDir, cursor: false });
    await page.goto("file://" + path.resolve(__dirname, "scenes/reel.html") + "?v=" + v);
    await hold(13000);
    const webm = await finish(`reel${v}_raw`);
    const mp4 = path.join(outDir, `reel${v}.mp4`);
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-ss", "0.6", "-i", webm, "-t", "12", "-an", "-vf", "scale=720:1280,fps=25,format=yuv420p", "-c:v", "libx264", "-preset", "veryfast", "-crf", "22", "-movflags", "+faststart", mp4]);
    console.log("wrote", mp4);
  }
})();
