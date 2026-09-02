import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  parseGlobal,
  clickIncrement,
  IDLE_FLUSH_WAIT_MS,
} from "./fixtures";

test.describe("pw-counter-photon", () => {
  test("pw-counter-photon-refetch-happy", async ({ browser }) => {
    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();

    await seedAuth(pageA, "anonymous", { reset_rate_limit: true });
    // Share seed cookie onto B via API on same origin after navigating once.
    await pageA.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(pageA);

    await seedAuth(pageB, "anonymous", { reset_rate_limit: false });
    await pageB.goto("/counter", { waitUntil: "domcontentloaded" });
    await waitForHydrated(pageB);

    const beforeB = await parseGlobal(pageB);
    await clickIncrement(pageA);
    await pageA.waitForTimeout(IDLE_FLUSH_WAIT_MS);
    await expect
      .poll(async () => parseGlobal(pageA), { timeout: 30_000 })
      .toBe(beforeB + 1);

    // B should refetch via Photon without local click.
    await expect
      .poll(async () => parseGlobal(pageB), { timeout: 45_000 })
      .toBe(beforeB + 1);

    await contextA.close();
    await contextB.close();
  });
});
