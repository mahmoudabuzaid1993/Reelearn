// Records the Flutter app (web build with the fake API) at phone size:
//   node demo/record_phone.cjs <outDir>
// Produces phone.mp4 (Sara's minute) and manager.mp4 (Khalid's team view), each with marks.
const { launch, hold } = require("./lib.cjs");
const P = require("./phone_lib.cjs");

const APP = process.env.APP || "http://localhost:8099/";
const outDir = process.argv[2];
// Recorded at exactly the size the phone has in the final frame, so it is never rescaled.
const W = 444;
const H = 961;
const CX = W / 2;
const swipeUp = (page) => P.swipe(page, -Math.round(H * 0.57), { x: CX, y0: Math.round(H * 0.74) });

async function open(name) {
  const s = await launch({ width: W, height: H, outDir });
  await s.page.goto(APP, { waitUntil: "networkidle" });
  await hold(2500);
  await P.enableSemantics(s.page);
  s.mark("start");
  return { ...s, name };
}

async function signIn(page, phone) {
  await P.fill(page, "Company code", "alwadi", { delay: 80 });
  await P.tap(page, "Find my company");
  await P.waitFor(page, "Mobile number");
  await hold(500);
  await P.fill(page, "Mobile number", phone, { delay: 75 });
  await P.tap(page, "Send my code on WhatsApp");
  await P.waitFor(page, "4-digit code");
  await hold(500);
  await P.fill(page, "4-digit code", "1234", { delay: 140 });
  await P.tap(page, "Continue", { exact: true });
}

async function sara() {
  const { page, mark, finish } = await open("phone");
  try {
    await signIn(page, "500000010");
    await P.waitFor(page, "Start my first minute");
    mark("welcome");
    await hold(1300);
    await P.tap(page, "Start my first minute");
    await P.waitFor(page, "Start today's minute");
    mark("today");
    await page.mouse.move(CX, 340, { steps: 12 });
    await hold(2600);
    await P.tap(page, "Start today's minute");
    mark("reel1");
    await page.mouse.move(W - 46, 250, { steps: 12 });
    await hold(4600);
    await swipeUp(page);
    mark("q1");
    await hold(1100);
    await P.tap(page, "Once at hello, once at goodbye");
    await hold(2300);
    await swipeUp(page);
    mark("reel2");
    await hold(3400);
    await swipeUp(page);
    mark("q2");
    await hold(900);
    await swipeUp(page); // gated: the stack refuses and shows the toast
    mark("gate");
    await hold(1900);
    await P.tap(page, "Mop it up quickly");
    mark("wrong");
    await hold(2800);
    await swipeUp(page);
    mark("review");
    await hold(1100);
    await P.tap(page, "60,000");
    await hold(1900);
    await swipeUp(page);
    mark("done");
    await page.mouse.move(CX, 480, { steps: 12 });
    await hold(3800);
    mark("end");
  } catch (e) {
    console.error("FAILED:", e.message);
    process.exitCode = 1;
  } finally {
    await finish("phone");
  }
}

async function manager() {
  const { page, mark, finish } = await open("manager");
  try {
    await signIn(page, "500000002");
    await P.waitFor(page, "Start today's minute");
    await hold(600);
    await P.tap(page, "Team", { exact: true });
    await P.waitFor(page, "Sara Al-Qahtani");
    mark("team");
    await page.mouse.move(CX, 380, { steps: 14 });
    await hold(2600);
    for (let i = 0; i < 20; i++) {
      await page.mouse.wheel(0, 14);
      await hold(22);
    }
    await hold(1800);
    await P.tap(page, "Nudge").catch(() => {});
    mark("nudged");
    await hold(2600);
    mark("end");
  } catch (e) {
    console.error("FAILED:", e.message);
    process.exitCode = 1;
  } finally {
    await finish("manager");
  }
}

(async () => {
  await sara();
  await manager();
})();
