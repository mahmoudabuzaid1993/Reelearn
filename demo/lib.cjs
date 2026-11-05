// Shared helpers for the demo recordings: a Chromium context that records video,
// a visible cursor (Playwright's mouse is invisible in recordings), human-paced
// clicks and typing, and a timeline of named marks for the edit.
const path = require("path");
const fs = require("fs");
const { chromium } = require(path.resolve(__dirname, "../web/node_modules/@playwright/test"));

const CURSOR_JS = `
(() => {
  if (window.__demoCursor) return;
  const make = () => {
    const c = document.createElement('div');
    c.id = '__demo_cursor';
    c.style.cssText = 'position:fixed;left:0;top:0;width:22px;height:22px;margin:-11px 0 0 -11px;border-radius:50%;background:rgba(255,98,87,.55);border:2px solid #fff;box-shadow:0 2px 8px rgba(0,0,0,.35);z-index:2147483647;pointer-events:none;transition:transform .08s ease,background .12s ease;transform:translate(-100px,-100px)';
    document.documentElement.appendChild(c);
    window.__demoCursor = c;
    let x = -100, y = -100;
    const place = (s) => { c.style.transform = 'translate(' + x + 'px,' + y + 'px) scale(' + s + ')'; };
    window.addEventListener('mousemove', (e) => { x = e.clientX; y = e.clientY; place(1); }, true);
    window.addEventListener('mousedown', () => { c.style.background = 'rgba(255,98,87,.95)'; place(0.75); }, true);
    window.addEventListener('mouseup', () => { c.style.background = 'rgba(255,98,87,.55)'; place(1); }, true);
  };
  if (document.documentElement) make(); else document.addEventListener('DOMContentLoaded', make);
})();`;

// Recording: Chrome's screencast, one JPEG per changed frame at viewport size. To get large, crisp
// footage the viewport is the final size (1920x1080 for the console with the page zoomed, the exact
// on-screen size for the phone) rather than upscaling afterwards.
// Frames arrive when the picture changes; each is written with its timestamp and the clip is
// assembled at a constant 30 fps when the session ends.
const { execFileSync } = require("child_process");

async function launch({ width, height, scale = 1, zoom = 1, outDir, cursor = true, locale = "en-US" }) {
  fs.mkdirSync(outDir, { recursive: true });
  const browser = await chromium.launch({ args: ["--autoplay-policy=no-user-gesture-required"] });
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: scale, locale });
  if (cursor) await context.addInitScript(CURSOR_JS);
  // `zoom` enlarges the page (body zoom keeps mouse coordinates and the cursor in viewport pixels);
  // the Next.js development badge is not part of the product.
  await context.addInitScript((zoom) => {
    const apply = () => {
      const st = document.createElement("style");
      st.textContent = (zoom !== 1 ? `body{zoom:${zoom}}` : "") + " nextjs-portal{display:none!important}";
      document.documentElement.appendChild(st);
    };
    if (document.documentElement) apply(); else document.addEventListener("DOMContentLoaded", apply);
  }, zoom);
  const page = await context.newPage();
  const client = await context.newCDPSession(page);
  const framesDir = fs.mkdtempSync(path.join(outDir, "frames-"));
  const frames = [];
  let first = null;
  let wallFirst = null;
  client.on("Page.screencastFrame", (f) => {
    const ts = f.metadata.timestamp;
    if (first === null) {
      first = ts;
      wallFirst = Date.now();
    }
    const file = path.join(framesDir, String(frames.length).padStart(6, "0") + ".jpg");
    fs.writeFileSync(file, Buffer.from(f.data, "base64"));
    frames.push({ file, t: ts - first });
    client.send("Page.screencastFrameAck", { sessionId: f.sessionId }).catch(() => {});
  });
  await client.send("Page.startScreencast", { format: "jpeg", quality: 90, everyNthFrame: 1 });
  const started = Date.now();
  const marks = [];
  const mark = (name) => {
    const t = (Date.now() - (wallFirst ?? started)) / 1000;
    marks.push({ name, t: Number(t.toFixed(2)) });
    console.log(`  ${t.toFixed(1).padStart(6)}s  ${name}`);
  };
  const finish = async (name) => {
    const endT = (Date.now() - (wallFirst ?? started)) / 1000;
    await client.send("Page.stopScreencast").catch(() => {});
    await context.close();
    await browser.close();
    const list = path.join(framesDir, "list.txt");
    const lines = [];
    frames.forEach((f, i) => {
      const next = i + 1 < frames.length ? frames[i + 1].t : Math.max(endT, f.t + 0.04);
      lines.push(`file '${f.file}'`, `duration ${Math.max(next - f.t, 0.001).toFixed(4)}`);
    });
    if (frames.length) lines.push(`file '${frames[frames.length - 1].file}'`);
    fs.writeFileSync(list, lines.join("\n") + "\n");
    const dest = path.join(outDir, `${name}.mp4`);
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", list, "-vf", "fps=30,scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p", "-c:v", "libx264", "-preset", "veryfast", "-crf", "15", dest]);
    fs.rmSync(framesDir, { recursive: true, force: true });
    fs.writeFileSync(path.join(outDir, `${name}.marks.json`), JSON.stringify(marks, null, 1));
    console.log(`saved ${dest} (${frames.length} frames)`);
    return dest;
  };
  return { browser, context, page, mark, finish };
}

const hold = (ms) => new Promise((r) => setTimeout(r, ms));

/** Glide the cursor to the element's centre, pause a beat, click. */
async function click(page, locator, { pause = 250 } = {}) {
  await locator.waitFor({ state: "visible", timeout: 30000 });
  await locator.scrollIntoViewIfNeeded();
  const box = await locator.boundingBox();
  if (box) {
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 24 });
    await hold(pause);
  }
  await locator.click();
}

/** Click into a field, clear it, and type like a person. */
async function type(page, locator, text, { delay = 28 } = {}) {
  await click(page, locator, { pause: 120 });
  await locator.fill("");
  await locator.pressSequentially(text, { delay });
}

/** Smooth vertical scroll of the page (or an element) by `dy` pixels. */
async function scroll(page, dy, { steps = 30, stepMs = 16 } = {}) {
  for (let i = 0; i < steps; i++) {
    await page.mouse.wheel(0, dy / steps);
    await hold(stepMs);
  }
}

module.exports = { launch, hold, click, type, scroll };
