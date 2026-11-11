import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("AI providers", () => {
  test("company admin routes the writer, sees whose key is used, and adds its own key", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/settings");
    await page.getByRole("tab", { name: "AI" }).click();

    await expect(page.getByRole("heading", { name: "Who does what" })).toBeVisible();
    await expect(page.getByTestId("access-ai_claude")).toContainText("Your key");
    await expect(page.getByTestId("access-ai_typesafe")).toContainText("Reelearn's key");
    await expect(page.getByTestId("access-asr")).toContainText("Not set up");
    await expect(page.getByTestId("ai-plan")).toContainText("this month");

    await page.getByLabel("Writer").selectOption("claude");
    await page.getByRole("button", { name: "Save" }).first().click();
    await expect(page.getByText("AI settings saved")).toBeVisible();

    await page.getByTestId("access-ai_typesafe").getByRole("button", { name: "Add your key" }).click();
    await page.getByLabel("API key").fill("ts-company-key");
    await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
    await expect(page.getByTestId("access-ai_typesafe")).toContainText("Your key");

    await expect(page.getByRole("cell", { name: "Checking" })).toBeVisible();
    await expect(page.getByText("On Reelearn's key")).toBeVisible();
  });

  test("platform operator sets a platform key and a company's AI plan", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/platform");
    await expect(page.getByRole("heading", { name: "Platform", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Platform", exact: true })).toBeVisible();

    await page.getByTestId("platform-ai_claude").getByRole("button", { name: "Configure" }).click();
    await page.getByLabel("API key").fill("sk-platform");
    await page.getByLabel(/Price per million input tokens/).fill("3");
    await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Platform key saved")).toBeVisible();
    await expect(page.getByTestId("platform-ai_claude")).toContainText("Connected");

    await page.getByRole("tab", { name: "Companies" }).click();
    const row = page.getByTestId("tenant-alwadi");
    await expect(row).toContainText("+20%");
    await row.getByRole("button", { name: "Edit plan" }).click();
    await page.getByLabel(/Markup on platform-key cost/).fill("900");
    await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Enter 0 to 500.")).toBeVisible();
    await page.getByLabel(/Markup on platform-key cost/).fill("35");
    await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Plan saved")).toBeVisible();
    await expect(row).toContainText("+35%");
    await expect(page.getByTestId("tenant-nakheel-clinics")).toContainText("Own keys only");
  });

  test("reviewers see the TypeSafe Jev check on drafts", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/library?status=in_review");
    await expect(page.getByText("52/100").first()).toBeVisible();
    await expect(page.getByText("Needs a careful look").first()).toBeVisible();
    await page.getByRole("link", { name: /ladder/i }).first().click();
    await expect(page.getByTestId("quality-card")).toContainText("Answer key looks wrong");
  });
});
