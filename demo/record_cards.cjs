// Records the animated cards and renders the framing graphics from demo/script.json.
//   node demo/record_cards.cjs <outDir>
const fs = require("fs");
const path = require("path");
const { launch, hold } = require("./lib.cjs");
const { chromium } = require(path.resolve(__dirname, "../web/node_modules/@playwright/test"));

const outDir = process.argv[2];
const script = JSON.parse(fs.readFileSync(path.join(__dirname, "script.json"), "utf8"));
const CARD = "file://" + path.resolve(__dirname, "scenes/card.html");
const FRAME = "file://" + path.resolve(__dirname, "scenes/frame.html");
const cardIds = { "01_title": "title", "02_problem": "problem", "04b_pipeline": "pipeline", "08_close": "close" };

(async () => {
  // Animated cards: recorded for the scene's length plus a second of tail.
  for (const scene of script.scenes.filter((s) => s.kind === "card")) {
    const { page, finish } = await launch({ width: 1920, height: 1080, scale: 1, outDir, cursor: false });
    await page.goto(`${CARD}?id=${cardIds[scene.id]}`);
    await page.evaluate(() => document.fonts.ready);
    await hold((scene.seconds + 1.2) * 1000);
    await finish(`card_${scene.id}`);
  }

  // Stills: phone backgrounds and lower-third titles.
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  for (const scene of script.scenes.filter((s) => s.kind !== "card")) {
    if (scene.kind === "phone") {
      const q = new URLSearchParams({ kind: "phonebg", eyebrow: scene.id.startsWith("06") ? "THE EMPLOYEE APP" : "MANAGERS AND HR", title: scene.title, bullets: (scene.bullets || []).join("|") });
      await page.goto(`${FRAME}?${q}`);
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({ path: path.join(outDir, `bg_${scene.id}.png`) });
    }
    const q = new URLSearchParams({ kind: "lower", title: scene.title });
    await page.goto(`${FRAME}?${q}`);
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: path.join(outDir, `lower_${scene.id}.png`), omitBackground: true });
  }
  await browser.close();
  console.log("cards and frames done");
})();
