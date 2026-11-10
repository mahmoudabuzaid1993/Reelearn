import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("learning engine", () => {
  test("switch preset, tune a weight, save as custom", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/engine");
    await expect(page.getByRole("heading", { name: "Learning engine" })).toBeVisible();
    await expect(page.getByRole("radio", { name: /Balanced/ })).toHaveAttribute("aria-checked", "true");

    await page.getByRole("radio", { name: /Intense/ }).click();
    await expect(page.getByText("Switched to Intense")).toBeVisible();
    await expect(page.getByRole("radio", { name: /Intense/ })).toHaveAttribute("aria-checked", "true");
    await expect(page.getByLabel("Practice per day")).toHaveValue("45");

    // Range inputs move by their step (5 minutes) with the arrow keys.
    await page.getByLabel("Practice per day").focus();
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await expect(page.getByLabel("Practice per day")).toHaveValue("30");
    await expect(page.getByText("1 unsaved change")).toBeVisible();
    await page.getByRole("button", { name: "Save changes" }).click();
    await expect(page.getByText("Engine settings saved")).toBeVisible();
    await expect(page.getByText("Custom").first()).toBeVisible();
    await expect(page.getByRole("radio", { name: /Intense/ })).toHaveAttribute("aria-checked", "false");
  });

  test("insights show practice, interest, reasons and reel performance", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/engine");
    await page.getByRole("tab", { name: "Insights" }).click();
    await expect(page.getByText("Practice minutes").first()).toBeVisible();
    await expect(page.getByRole("cell", { name: "Customer service", exact: true })).toBeVisible();
    await expect(page.getByText("Sent by a colleague")).toBeVisible();
    await expect(page.getByText("Greet within 10 seconds")).toBeVisible();
    await expect(page.getByText(/fewer than three people/)).toBeVisible();
  });
});
