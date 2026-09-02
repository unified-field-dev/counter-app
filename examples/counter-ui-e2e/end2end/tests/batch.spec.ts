import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  parseGlobal,
  clickIncrement,
  IDLE_FLUSH_WAIT_MS,
} from "./fixtures";

test.describe("pw-counter-batch", () => {
  test("pw-counter-live-batch-idle-happy", async ({ page }) => {
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const before = await parseGlobal(page);
    for (let i = 0; i < 5; i++) {
      await clickIncrement(page);
    }
    await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(page), { timeout: 30_000 })
      .toBe(before + 5);
  });

  test("pw-counter-live-batch-max-age-happy", async ({ page }) => {
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const before = await parseGlobal(page);

    // Click every ~800ms for >5s so idle (1s) never fires alone; max-age (5s)
    // must flush. Count clicks so we can assert the full amount lands.
    let clicks = 0;
    const end = Date.now() + 5_500;
    while (Date.now() < end) {
      await clickIncrement(page);
      clicks += 1;
      await page.waitForTimeout(800);
    }
    // Drain any trailing idle flush.
    await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(page), { timeout: 30_000 })
      .toBe(before + clicks);
  });
});
