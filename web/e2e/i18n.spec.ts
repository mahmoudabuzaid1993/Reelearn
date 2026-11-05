import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("Arabic locale", () => {
  test("switches the console to RTL Arabic and back", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
    await page.getByRole("button", { name: "Switch language to ar" }).click();
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator("html")).toHaveAttribute("lang", "ar");
    await expect(page.getByRole("heading", { name: "لوحة المتابعة" })).toBeVisible();
    await expect(page.getByRole("link", { name: "المكتبة" })).toBeVisible();

    await page.goto("/library");
    await expect(page.getByRole("heading", { name: "مكتبة الريلز" })).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");

    await page.getByRole("button", { name: "تبديل اللغة إلى en" }).click();
    await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
  });
});
