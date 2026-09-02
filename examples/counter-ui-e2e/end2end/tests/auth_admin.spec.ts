import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  parseGlobal,
  parseUser,
  clickIncrement,
  IDLE_FLUSH_WAIT_MS,
} from "./fixtures";

test.describe("pw-counter-auth-admin", () => {
  test("pw-counter-admin-set-happy", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Expand navigation" }).click();
    await page.getByTestId("nav-counter-admin").click();
    await expect(page.getByTestId("counter-admin-container")).toBeVisible({
      timeout: 120_000,
    });
    const target = 42;
    await page.getByTestId("set-input").locator("input").fill(String(target));
    await page.getByTestId("set-submit").getByRole("button").click();
    await expect
      .poll(
        async () => {
          const body = page.getByTestId("counter-admin-container");
          const text = (await body.textContent()) ?? "";
          return text.includes(String(target));
        },
        { timeout: 30_000 },
      )
      .toBe(true);
  });

  test("pw-counter-user-incr-happy", async ({ page }) => {
    await seedAuth(page, "owner", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("user-counter")).toBeVisible({
      timeout: 30_000,
    });
    const beforeGlobal = await parseGlobal(page);
    const beforeUser = await parseUser(page);
    await clickIncrement(page);
    await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(page), { timeout: 30_000 })
      .toBe(beforeGlobal + 1);
    await expect
      .poll(async () => parseUser(page), { timeout: 30_000 })
      .toBe(beforeUser + 1);
  });

  test("pw-counter-admin-gate-sad", async ({ page }) => {
    await seedAuth(page, "anonymous");
    await page.goto("/counter/admin", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("auth-required-empty-state")).toBeAttached({
      timeout: 60_000,
    });
    await expect(page.getByTestId("counter-admin-container")).toHaveCount(0);
  });

  test("pw-counter-admin-gate-unverified-sad", async ({ page }) => {
    await seedAuth(page, "unverified");
    await page.goto("/counter/admin", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(
      page.getByTestId("email-verification-required-empty-state"),
    ).toBeAttached({ timeout: 60_000 });
    await expect(page.getByTestId("counter-admin-container")).toHaveCount(0);
  });

  test("pw-counter-admin-perm-sad", async ({ page }) => {
    await seedAuth(page, "member");
    await page.goto("/counter/admin", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("permission-required-empty-state")).toBeAttached({
      timeout: 60_000,
    });
    await expect(page.getByTestId("counter-admin-container")).toHaveCount(0);
  });
});
