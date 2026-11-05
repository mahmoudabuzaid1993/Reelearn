import { expect, type Page } from "@playwright/test";

export const DEMO = {
  admin: "admin@alwadi.example",
  sme: "hana@alwadi.example",
  code: "1234",
  handbookSourceId: "00000000-0000-4000-8000-000600000001",
};

/** Reset the persisted mock db so each spec starts from the seed. */
export async function resetMockDb(page: Page) {
  await page.goto("/signin");
  await page.evaluate(() => window.localStorage.removeItem("reelearn.mockdb.v1"));
}

export async function signIn(page: Page, email: string) {
  await page.goto("/signin");
  await page.getByLabel("Work email").fill(email);
  await page.getByRole("button", { name: "Send code" }).click();
  await page.getByLabel("One-time code").fill(DEMO.code);
  await page.getByRole("button", { name: "Verify and sign in" }).click();
  await expect(page).toHaveURL(/\/dashboard/);
}

export async function signOut(page: Page) {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page).toHaveURL(/\/signin/);
}
