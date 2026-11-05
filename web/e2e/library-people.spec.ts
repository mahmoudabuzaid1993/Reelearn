import { expect, test } from "@playwright/test";
import { DEMO, resetMockDb, signIn } from "./helpers";

test.describe("library and people", () => {
  test("library filters by status and search, and shows awaiting states", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/library");
    await expect(page.getByRole("link", { name: "Say the customer's name twice in the first minute" })).toBeVisible();
    await expect(page.getByText("Awaiting SME sign-off")).toBeVisible();

    await page.getByLabel("Status").selectOption("in_review");
    await expect(page).toHaveURL(/status=in_review/);
    await expect(page.getByRole("link", { name: "Greet within ten seconds" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Say the customer's name twice in the first minute" })).toHaveCount(0);

    await page.getByLabel("Status").selectOption("");
    await page.getByLabel("Search").fill("cash");
    await expect(page.getByRole("link", { name: "Which cash transactions must be reported" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Greet within ten seconds" })).toHaveCount(0);

    await page.getByLabel("Search").fill("zzzz");
    await expect(page.getByText("No reels match")).toBeVisible();
  });

  test("people CSV import shows a dry-run preview with per-row errors before importing", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/people");
    await page.getByRole("button", { name: "Import CSV" }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Or paste CSV").fill(`full_name,role,phone,email,language
Layla Hassan,learner,0551234567,,ar
,learner,0559999999,,ar
Bad Role,ceo,0558888888,,ar
Existing Admin,admin,,admin@alwadi.example,ar`);
    await expect(dialog.getByText("4 rows")).toBeVisible();
    await expect(dialog.getByText("1 to create")).toBeVisible();
    await expect(dialog.getByText("1 to update")).toBeVisible();
    await expect(dialog.getByText("2 skipped")).toBeVisible();
    await expect(dialog.getByText("name missing")).toBeVisible();
    await expect(dialog.getByText("unknown role")).toBeVisible();

    const importButton = dialog.getByRole("button", { name: "Import", exact: true });
    await expect(importButton).toBeDisabled();
    await dialog.getByRole("button", { name: "Dry run" }).click();
    await expect(dialog.getByText("Server dry run finished. Nothing was written.")).toBeVisible();
    await expect(importButton).toBeEnabled();
    await importButton.click();
    await expect(page.getByText("Imported: 1 created, 1 updated")).toBeVisible();
    await expect(page.getByText("Layla Hassan")).toBeVisible();
  });

  test("topics matrix toggles an assignment and saves", async ({ page }) => {
    await resetMockDb(page);
    await signIn(page, DEMO.admin);
    await page.goto("/topics");
    const cell = page.getByLabel("Assign AML to Store 12 · Jeddah");
    await expect(cell).not.toBeChecked();
    await cell.check();
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Assignments saved")).toBeVisible();
    await page.reload();
    await expect(page.getByLabel("Assign AML to Store 12 · Jeddah")).toBeChecked();
  });
});
