import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  parseGlobal,
  clickIncrement,
  IDLE_FLUSH_WAIT_MS,
} from "./fixtures";

test.describe("pw-counter-live", () => {
  test("pw-counter-lab-boot-happy", async ({ page }) => {
    await seedAuth(page, "anonymous");
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("counter-container")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("global-counter")).toBeVisible();
  });

  test("pw-counter-live-anon-incr-happy", async ({ page }) => {
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const before = await parseGlobal(page);
    await clickIncrement(page);
    await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(page), { timeout: 30_000 })
      .toBe(before + 1);
  });

  test("pw-counter-live-validation-sad", async ({ page }) => {
    // The live IncrementButton only dispatches positive pending click counts.
    // Invalid amounts (0 / over max) are covered by counter-app-worker unit +
    // server_fn mapping tests — no accessible invalid-amount UI control here.
    test.skip(
      true,
      "validation is server-covered; UI cannot dispatch invalid amount",
    );
    await seedAuth(page, "anonymous");
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
  });

  test("pw-counter-anon-rate-limit-sad", async ({ page }) => {
    // Host defaults COUNTER_ANON_INCREMENTS_PER_MIN=3; seed clears buckets.
    await seedAuth(page, "anonymous", { reset_rate_limit: true });
    await page.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);

    let last = await parseGlobal(page);
    let sawRateLimit = false;
    for (let i = 0; i < 8; i++) {
      await clickIncrement(page);
      await page.waitForTimeout(IDLE_FLUSH_WAIT_MS);
      const errBar = page.getByText(/increment failed:.*rate limit/i);
      if ((await errBar.count()) > 0) {
        sawRateLimit = true;
        last = await parseGlobal(page);
        break;
      }
      const next = await parseGlobal(page);
      last = next;
    }
    expect(sawRateLimit).toBe(true);
    expect(last).toBeGreaterThanOrEqual(3);
    await page.waitForTimeout(500);
    expect(await parseGlobal(page)).toBe(last);
  });
});
