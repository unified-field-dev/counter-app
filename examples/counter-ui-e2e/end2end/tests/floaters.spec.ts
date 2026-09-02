import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  parseGlobal,
  clickIncrement,
  IDLE_FLUSH_WAIT_MS,
} from "./fixtures";

test.describe("pw-counter-floaters", () => {
  test("pw-counter-floater-spawn-happy", async ({ page }) => {
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const before = await parseGlobal(page);
    await clickIncrement(page);
    await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(page), { timeout: 30_000 })
      .toBe(before + 1);
    const floater = page.getByTestId("delta-floater").first();
    await expect(floater).toBeAttached({ timeout: 15_000 });
    await expect(floater).toHaveText("+1");
    await expect(floater).toHaveAttribute("data-amount", "1");
  });

  test("pw-counter-floater-variance-happy", async ({ page }) => {
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);

    const signatures = new Set<string>();
    for (let i = 0; i < 3; i++) {
      await clickIncrement(page);
      await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
      const floaters = page.getByTestId("delta-floater");
      const count = await floaters.count();
      for (let j = 0; j < count; j++) {
        const el = floaters.nth(j);
        const left = (await el.getAttribute("data-left")) ?? "";
        const top = (await el.getAttribute("data-top")) ?? "";
        const rot = (await el.getAttribute("data-rotation")) ?? "";
        signatures.add(`${left}|${top}|${rot}`);
      }
    }
    expect(signatures.size).toBeGreaterThan(1);
  });
});
