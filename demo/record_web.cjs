// Records the company console: sign-in, dashboard, the AI Reel Studio flow, dual approval,
// library and reports. Runs against the web app at WEB (default http://localhost:3000).
//   node demo/record_web.cjs <outDir> [pace]
// `pace` scales every deliberate pause (1 = demo speed, 0.1 = quick trial).
const path = require("path");
const { launch, hold: rawHold, click, type, scroll } = require("./lib.cjs");

const WEB = process.env.WEB || "http://localhost:3000";
const outDir = process.argv[2] || path.resolve(__dirname, "out");
const pace = Number(process.argv[3] || 1);
const hold = (ms) => rawHold(ms * pace);
const ADMIN = "admin@alwadi.example";
const SME = "hana@alwadi.example";

async function signIn(page, email, mark) {
  await page.goto(`${WEB}/signin`);
  await page.getByLabel("Work email").waitFor();
  await hold(600);
  await type(page, page.getByLabel("Work email"), email);
  await click(page, page.getByRole("button", { name: "Send code" }));
  await type(page, page.getByLabel("One-time code"), "1234", { delay: 90 });
  await click(page, page.getByRole("button", { name: "Verify and sign in" }));
  await page.waitForURL(/\/dashboard/, { timeout: 30000 });
  if (mark) mark(`signed in as ${email}`);
}

async function signOut(page) {
  await click(page, page.getByRole("button", { name: "Sign out" }));
  await page.waitForURL(/\/signin/);
}

async function shot(page, name) {
  if (process.env.SHOTS) await page.screenshot({ path: path.join(outDir, `shot_${name}.png`) });
}

(async () => {
  const { page, mark, finish } = await launch({ width: 1920, height: 1080, zoom: 1.5, outDir });
  try {
    // ---- console: sign in and dashboard
    mark("start");
    await signIn(page, ADMIN, mark);
    await page.getByRole("heading").first().waitFor();
    await hold(1500);
    mark("dashboard");
    await shot(page, "dashboard");
    await page.mouse.move(495, 375, { steps: 30 });
    await hold(1200);
    await page.mouse.move(1140, 375, { steps: 30 });
    await hold(1200);
    await page.mouse.move(1470, 660, { steps: 30 });
    await hold(2600);
    await page.mouse.move(1470, 760, { steps: 24 });
    await hold(2600);

    // ---- studio: describe a topic
    await page.goto(`${WEB}/studio/new`);
    mark("studio_new");
    await hold(1200);
    await shot(page, "studio_new");
    await click(page, page.getByRole("tab", { name: "Describe a topic" }));
    await type(page, page.getByLabel("Topic brief"), "Ramadan trading hours: when stores open, break times, and what to tell customers at the door.", { delay: 14 });
    await type(page, page.getByLabel("Title"), "Ramadan trading hours");
    await hold(500);
    await click(page, page.getByRole("button", { name: "Continue" }));
    await page.waitForURL(/\/studio\/jobs\/new\?source=/, { timeout: 30000 });
    mark("setup");
    await page.getByTestId("quote-total").waitFor({ timeout: 30000 });
    await hold(1200);
    // Customer service topic; English narration first, Arabic as the dubbed variant.
    const topic = page.getByRole("combobox", { name: /Topic/ }).first();
    await click(page, topic);
    await topic.selectOption({ label: "Customer service" });
    await hold(700);
    await click(page, page.getByLabel("Arabic", { exact: true }));
    await hold(300);
    await click(page, page.getByLabel("Arabic", { exact: true }));
    await hold(1500);
    await shot(page, "setup");
    const quote = page.getByTestId("quote-total");
    const qb = await quote.boundingBox();
    if (qb) await page.mouse.move(qb.x + qb.width / 2, qb.y + qb.height / 2, { steps: 30 });
    await hold(2500);
    mark("quote");
    await click(page, page.getByLabel("I have checked the quote and want to start"));
    await click(page, page.getByRole("button", { name: "Start generation" }));
    await page.waitForURL(/\/studio\/jobs\/[0-9a-f-]+$/, { timeout: 30000 });
    mark("generating");
    await page.getByRole("progressbar").waitFor({ timeout: 30000 });
    await shot(page, "generating");
    await page.waitForURL(/\/storyboard$/, { timeout: 120000 });
    mark("storyboard");
    await page.getByRole("heading", { name: "Storyboard" }).waitFor();
    await hold(2500);
    await shot(page, "storyboard");
    await scroll(page, 630);
    await hold(2200);
    await scroll(page, -630);
    await click(page, page.getByRole("button", { name: "Send all to review" }));
    await hold(1500);
    mark("sent_to_review");
    await click(page, page.getByRole("link", { name: "Edit" }).first());
    await page.waitForURL(/\/studio\/reels\/[0-9a-f-]+$/, { timeout: 30000 });
    const reelUrl = page.url();
    mark("editor");
    await hold(1800);
    await shot(page, "editor");
    await type(page, page.getByLabel("Script"), "Ramadan hours change at the door. Say the new opening time once, calmly, and point to the sign. One sentence is enough.", { delay: 12 });
    await click(page, page.getByRole("button", { name: "Save", exact: true }));
    await hold(2500);
    mark("saved");
    await shot(page, "editor_saved");

    // ---- approvals: publish is blocked until L&D and the subject expert both approve
    await click(page, page.getByRole("link", { name: "Approve and publish" }));
    await page.waitForURL(/\/publish$/, { timeout: 30000 });
    mark("publish_blocked");
    await hold(2500);
    await shot(page, "publish_blocked");
    await click(page, page.getByTestId("approval-ld").getByRole("button", { name: "Approve" }));
    await hold(2200);
    mark("ld_approved");
    await shot(page, "ld_approved");
    await signOut(page);
    await signIn(page, SME, mark);
    await page.goto(`${reelUrl}/publish`);
    await hold(1200);
    await click(page, page.getByTestId("approval-sme").getByRole("button", { name: "Approve" }));
    await hold(2000);
    mark("sme_approved");
    await shot(page, "sme_approved");
    await signOut(page);
    await signIn(page, ADMIN, mark);
    await page.goto(`${reelUrl}/publish`);
    await hold(1000);
    const groups = page.getByRole("checkbox");
    await click(page, groups.first());
    await hold(800);
    await shot(page, "ready_to_publish");
    await click(page, page.getByTestId("publish-button"));
    await page.waitForURL(/\/library/, { timeout: 30000 });
    mark("published");
    await hold(3000);
    await shot(page, "library");

    // ---- reports
    await page.goto(`${WEB}/reports`);
    mark("reports");
    await hold(3500);
    await shot(page, "reports");
    await scroll(page, 540);
    await hold(2500);
    mark("end");
  } catch (e) {
    console.error("FAILED:", e.message.split("\n")[0]);
    await page.screenshot({ path: path.join(outDir, "failure.png") }).catch(() => {});
    process.exitCode = 1;
  } finally {
    await finish("web");
  }
})();
