import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("settings, reports, audit, onboarding, signup", () => {
  test("settings shows integrations, residency and credits from fixtures", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/settings");
    await expect(page.getByText("Jisr (HRIS)")).toBeVisible();
    await expect(page.getByText("Quota exceeded for project (429)")).toBeVisible();
    await page.getByRole("tab", { name: "Residency" }).click();
    await expect(page.getByText("Saudi Arabia")).toBeVisible();
    await page.getByRole("tab", { name: "Credits" }).click();
    await expect(page.getByText("1,200")).toBeVisible();
  });

  test("reports: by topic, by group, HRDF export job", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/reports");
    await expect(page.getByRole("cell", { name: "Customer service" })).toBeVisible();
    await page.getByRole("tab", { name: "By group" }).click();
    await expect(page.getByRole("cell", { name: "Jeddah warehouse" })).toBeVisible();
    await page.getByRole("tab", { name: "HRDF evidence" }).click();
    await page.getByRole("button", { name: "Build pack" }).click();
    await expect(page.getByText("Evidence pack queued")).toBeVisible();
    await expect(page.getByText("Queued").first()).toBeVisible();
    await expect(page.getByText("Ready").nth(1)).toBeVisible({ timeout: 15_000 });
  });

  test("audit log lists writes and filters by entity", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/audit");
    await expect(page.getByText("reel.published")).toBeVisible();
    await page.getByLabel("Entity").selectOption("user");
    await expect(page.getByText("user.invited")).toBeVisible();
    await expect(page.getByText("reel.published")).toHaveCount(0);
  });

  test("signup validates and hands off to OTP; onboarding steps render", async ({ page }) => {
    await resetMockDb(page);
    await page.goto("/signup");
    await page.getByRole("button", { name: "Create workspace" }).click();
    await expect(page.getByRole("alert").first()).toBeVisible();
    await page.getByLabel("Company name").fill("Noor Pharmacies");
    await expect(page.getByLabel("Workspace code")).toHaveValue("noor-pharmacies");
    await page.getByLabel("Your name").fill("Lina");
    await page.getByLabel("Your work email").fill("lina@noor.example");
    await page.getByRole("button", { name: "Create workspace" }).click();
    await expect(page).toHaveURL(/\/signin\?challenge=/);
    await page.getByLabel("One-time code").fill("1234");
    await page.getByRole("button", { name: "Verify and sign in" }).click();
    await expect(page).toHaveURL(/\/dashboard/);

    await page.goto("/onboarding/topics");
    await expect(page.getByRole("heading", { name: "Topics and people" })).toBeVisible();
    await expect(page.getByText("Added").first()).toBeVisible();
    await page.getByRole("button", { name: "Continue to delivery" }).click();
    await expect(page).toHaveURL(/\/onboarding\/delivery/);
    await page.getByRole("button", { name: "Save and open the dashboard" }).click();
    await expect(page).toHaveURL(/\/dashboard/);
  });
});
