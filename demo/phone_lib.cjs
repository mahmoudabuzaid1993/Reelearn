// Driving the Flutter web build with Playwright: Flutter paints to a canvas, so the
// semantics tree is switched on and widgets are addressed by their accessible text.
const { hold } = require("./lib.cjs");

async function enableSemantics(page) {
  await page.evaluate(() => {
    const b = document.querySelector("flt-semantics-placeholder");
    if (b) b.click();
  });
  await hold(600);
}

/** Leaf texts currently on screen (for exploring a screen). */
async function labels(page) {
  return page.evaluate(() => {
    const out = [];
    for (const e of document.querySelectorAll("flt-semantics")) {
      const own = Array.from(e.childNodes).filter((n) => n.nodeType === 3).map((n) => n.textContent.trim()).join(" ");
      const label = e.getAttribute("aria-label") || own;
      const input = e.querySelector(":scope > input, :scope > textarea");
      if (label || input) out.push(`${e.getAttribute("role") || (input ? "input" : "text")}: ${label}${input ? " [" + (input.getAttribute("aria-label") || input.value || "") + "]" : ""}`);
    }
    return Array.from(new Set(out));
  });
}

/** Centre of the first semantics node whose text or aria-label matches. */
async function locate(page, text, { exact = false, nth = 0 } = {}) {
  const box = await page.evaluate(
    ({ text, exact, nth }) => {
      const nodes = Array.from(document.querySelectorAll("flt-semantics"));
      const matches = nodes.filter((e) => {
        const own = Array.from(e.childNodes).filter((n) => n.nodeType === 3).map((n) => n.textContent.trim()).join(" ");
        const input = e.querySelector(":scope > input, :scope > textarea");
        const label = e.getAttribute("aria-label") || own || (input ? input.getAttribute("aria-label") || "" : "");
        return exact ? label === text : label.includes(text);
      });
      // Prefer the smallest matching node (the control itself rather than a group around it).
      matches.sort((a, b) => a.getBoundingClientRect().width * a.getBoundingClientRect().height - b.getBoundingClientRect().width * b.getBoundingClientRect().height);
      const e = matches[nth];
      if (!e) return null;
      const r = e.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2, w: r.width, h: r.height };
    },
    { text, exact, nth },
  );
  return box;
}

async function waitFor(page, text, { timeout = 15000, exact = false } = {}) {
  const start = Date.now();
  for (;;) {
    const box = await locate(page, text, { exact });
    if (box) return box;
    if (Date.now() - start > timeout) throw new Error(`"${text}" did not appear`);
    await hold(150);
  }
}

/** Glide a finger-sized cursor to the widget and tap it. */
async function tap(page, text, opts = {}) {
  const box = await waitFor(page, text, opts);
  await page.mouse.move(box.x, box.y, { steps: 18 });
  await hold(opts.pause ?? 220);
  await page.mouse.down();
  await hold(70);
  await page.mouse.up();
  await hold(opts.after ?? 350);
}

async function tapAt(page, x, y, { pause = 200, after = 350 } = {}) {
  await page.mouse.move(x, y, { steps: 18 });
  await hold(pause);
  await page.mouse.down();
  await hold(70);
  await page.mouse.up();
  await hold(after);
}

/** Tap a text field (by its label/hint) and type. */
async function fill(page, text, value, { delay = 70 } = {}) {
  await tap(page, text, { after: 250 });
  await page.keyboard.type(value, { delay });
  await hold(250);
}

/** A thumb swipe: press, drag with easing, release. Negative dy swipes up. */
async function swipe(page, dy, { x = 195, y0 = 620, ms = 420 } = {}) {
  await page.mouse.move(x, y0, { steps: 10 });
  await hold(120);
  await page.mouse.down();
  const steps = 24;
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const eased = 1 - Math.pow(1 - t, 3);
    await page.mouse.move(x, y0 + dy * eased);
    await hold(ms / steps);
  }
  await page.mouse.up();
  await hold(650);
}

module.exports = { enableSemantics, labels, locate, waitFor, tap, tapAt, fill, swipe };
