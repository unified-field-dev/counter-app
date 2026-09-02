import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-counter-leaderboard", () => {
  test("pw-counter-leaderboard-page-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "anonymous", {
      seed_scores: true,
      reset_rate_limit: true,
    });
    const seededScores = [...(seeded.fixtures.scores ?? [])].sort(
      (a, b) => b.score - a.score,
    );
    expect(seededScores.length).toBeGreaterThanOrEqual(3);

    await page.goto("/counter/high-scores", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("high-scores-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect
      .poll(async () => page.getByTestId("counter-high-score-row").count(), {
        timeout: 120_000,
      })
      .toBeGreaterThanOrEqual(3);

    const rows = page.getByTestId("counter-high-score-row");
    const count = await rows.count();
    expect(count).toBeGreaterThanOrEqual(3);

    const scores: number[] = [];
    for (let i = 0; i < Math.min(count, 3); i++) {
      const raw = (await rows.nth(i).getAttribute("data-score")) ?? "";
      const score = Number.parseInt(raw, 10);
      expect(Number.isFinite(score)).toBeTruthy();
      scores.push(score);
    }

    // Seeded alice/bob/carol are 30/20/10 — page must show descending scores.
    expect(scores[0]).toBeGreaterThanOrEqual(scores[1]!);
    expect(scores[1]).toBeGreaterThanOrEqual(scores[2]!);
    expect(scores[0]).toBe(seededScores[0]!.score);
    expect(scores[1]).toBe(seededScores[1]!.score);
    expect(scores[2]).toBe(seededScores[2]!.score);
  });
});
