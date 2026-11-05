import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn, signOut } from "./helpers";

test.describe("studio: source -> quote -> job -> storyboard -> editor -> publish", () => {
  test("runs the full flow with approval gating", async ({ page }) => {
    test.setTimeout(120_000);
    await resetMockDb(page);
    await signIn(page, DEMO.admin);

    // S01: pick a source by describing a topic
    await page.goto("/studio/new");
    await page.getByRole("tab", { name: "Describe a topic" }).click();
    await page.getByLabel("Topic brief").fill("Ramadan trading hours: when stores open, break times, and what to tell customers at the door.");
    await page.getByLabel("Title").fill("Ramadan trading hours");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page).toHaveURL(/\/studio\/jobs\/new\?source=/);

    // S02: quote appears before confirming; 3 reels x 2 languages x avatar = 63 credits
    await expect(page.getByTestId("quote-total")).toHaveText("63 credits");
    const start = page.getByRole("button", { name: "Start generation" });
    await expect(start).toBeDisabled();
    await page.getByLabel("I have checked the quote and want to start").check();
    await expect(start).toBeEnabled();
    await start.click();

    // S03: live progress, then auto-open storyboard
    await expect(page).toHaveURL(/\/studio\/jobs\/[0-9a-f-]+$/);
    await expect(page.getByRole("progressbar")).toBeVisible();
    await expect(page.getByText("3 reels ready for review")).toBeVisible({ timeout: 30_000 });
    await expect(page).toHaveURL(/\/storyboard$/, { timeout: 15_000 });

    // S04: storyboard lists the reels; open the first editor
    await expect(page.getByRole("heading", { name: "Storyboard" })).toBeVisible();
    await page.getByRole("button", { name: "Send all to review" }).click();
    await expect(page.getByText("3 reels sent to review")).toBeVisible();
    await page.getByRole("link", { name: "Edit" }).first().click();
    await expect(page).toHaveURL(/\/studio\/reels\/[0-9a-f-]+$/);
    const reelUrl = page.url();

    // S06: edit the script, preview updates, save
    const script = page.getByLabel("Script");
    await script.fill("Ramadan hours change at the door. Say the new opening time once, calmly, and point to the sign. One sentence is enough.");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Reel saved")).toBeVisible();
    await expect(page.getByRole("img", { name: "Phone preview" })).toContainText("Ramadan hours change at the door.");

    // S07: publish blocked until both approvals
    await page.getByRole("link", { name: "Approve and publish" }).click();
    await expect(page).toHaveURL(/\/publish$/);
    const publish = page.getByTestId("publish-button");
    await expect(publish).toBeDisabled();
    await expect(page.getByText("Subject expert approval is missing")).toBeVisible();
    await page.getByLabel("Store 12 · Jeddah (3)").check();
    await page.getByTestId("approval-ld").getByRole("button", { name: "Approve" }).click();
    await expect(page.getByTestId("approval-ld")).toContainText("Approved");
    await expect(publish).toBeDisabled();

    // SME signs in and approves the same version
    await signOut(page);
    await signIn(page, DEMO.sme);
    await page.goto(`${reelUrl}/publish`);
    await page.getByTestId("approval-sme").getByRole("button", { name: "Approve" }).click();
    await expect(page.getByTestId("approval-sme")).toContainText("Approved");

    // Back as admin: publish is now enabled and succeeds
    await signOut(page);
    await signIn(page, DEMO.admin);
    await page.goto(`${reelUrl}/publish`);
    await page.getByLabel("Store 12 · Jeddah (3)").check();
    await expect(page.getByText("Ready to publish.")).toBeVisible();
    await expect(publish).toBeEnabled();
    await publish.click();
    await expect(page).toHaveURL(/\/library\?status=published/);
    await expect(page.getByRole("link", { name: "Ramadan trading hours: the one rule" })).toBeVisible();
  });

  test("retries only the failed step of a failed job", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/studio/jobs/00000000-0000-4000-8000-000700000002");
    await expect(page.getByText("Failed at: Render video")).toBeVisible();
    await page.getByRole("button", { name: "Retry this step" }).click();
    await expect(page.getByText("2 reels ready for review")).toBeVisible({ timeout: 30_000 });
  });

  test("podcast cut requires speaker consent", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/studio/sources/00000000-0000-4000-8000-000600000002/moments");
    await page.getByRole("button", { name: /Three points of contact/ }).click();
    await page.getByRole("button", { name: "Cut into a draft reel" }).click();
    await expect(page.getByText("Confirm the speaker's consent first.")).toBeVisible();
    await page.getByLabel("The speaker consents to this cut being published").check();
    await page.getByRole("button", { name: "Cut into a draft reel" }).click();
    await expect(page).toHaveURL(/\/studio\/reels\/[0-9a-f-]+$/);
    await expect(page.getByRole("heading", { name: "Three points of contact" })).toBeVisible();
  });
});
