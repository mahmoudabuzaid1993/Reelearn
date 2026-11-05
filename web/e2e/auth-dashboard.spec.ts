import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("sign in and dashboard", () => {
  test("redirects anonymous admins to sign in, then shows fixture numbers", async ({ page }) => {
    await resetMockDb(page);
    await page.goto("/dashboard");
    await expect(page).toHaveURL(/\/signin\?next=%2Fdashboard/);

    await signIn(page, DEMO.admin);
    await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
    await expect(page.getByText("71%")).toBeVisible();
    await expect(page.getByText("1,200")).toBeVisible();
    await expect(page.getByText("2 reels are waiting for SME sign-off")).toBeVisible();
    await expect(page.getByText("Noura Al-Harbi")).toBeVisible();
  });

  test("rejects a wrong code with an inline problem and request id", async ({ page }) => {
    await resetMockDb(page);
    await page.goto("/signin");
    await page.getByLabel("Work email").fill(DEMO.admin);
    await page.getByRole("button", { name: "Send code" }).click();
    await page.getByLabel("One-time code").fill("9999");
    await page.getByRole("button", { name: "Verify and sign in" }).click();
    await expect(page.getByText("The code does not match")).toBeVisible();
    await expect(page.getByText(/req_mock_/)).toBeVisible();
  });
});
