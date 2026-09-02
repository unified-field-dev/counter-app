import { test as base, expect, type Page } from "@playwright/test";

export type SeedAuthKind = "anonymous" | "owner" | "member" | "unverified";

export type SeedOpts = {
  seed_scores?: boolean;
  reset_rate_limit?: boolean;
};

export async function seedAuth(
  page: Page,
  auth: SeedAuthKind,
  opts?: SeedOpts,
) {
  const res = await page.request.post("/api/test/seed-data", {
    data: {
      auth,
      seed_scores: opts?.seed_scores ?? false,
      reset_rate_limit: opts?.reset_rate_limit ?? true,
    },
  });
  if (!res.ok()) {
    const body = await res.text();
    throw new Error(
      `seed-data failed: status=${res.status()} body=${body.slice(0, 2_000)}`,
    );
  }
  return res.json() as Promise<{
    ok: boolean;
    auth: string;
    fixtures: {
      global_value?: number | null;
      scores?: Array<{ user_id: string; score: number }> | null;
    };
  }>;
}

/**
 * Wait for Orbital boot overlay to finish and hydrate to mark the document ready.
 */
export async function waitForHydrated(page: Page, timeoutMs = 240_000) {
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const html = document.documentElement;
          if (html.getAttribute("data-orbital-boot-state") === "error") {
            return "error";
          }
          if (html.getAttribute("data-orbital-hydrated") === "true") {
            return "ready";
          }
          return "loading";
        }),
      { timeout: timeoutMs },
    )
    .not.toBe("error");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () =>
            document.documentElement.getAttribute("data-orbital-hydrated") ===
            "true",
        ),
      { timeout: timeoutMs },
    )
    .toBe(true);
  await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("e2e-auth-bootstrap")).toBeAttached({
    timeout: 30_000,
  });
}

export async function parseGlobal(page: Page): Promise<number> {
  const text = (
    (await page.getByTestId("global-counter").textContent()) ?? ""
  ).trim();
  const n = Number.parseInt(text.replace(/[^\d]/g, ""), 10);
  if (Number.isNaN(n)) {
    throw new Error(`global-counter not numeric: "${text}"`);
  }
  return n;
}

export async function parseUser(page: Page): Promise<number> {
  const text = (
    (await page.getByTestId("user-count").textContent()) ?? ""
  ).trim();
  const n = Number.parseInt(text.replace(/[^\d]/g, ""), 10);
  if (Number.isNaN(n)) {
    throw new Error(`user-count not numeric: "${text}"`);
  }
  return n;
}

export async function clickIncrement(page: Page) {
  await page.getByTestId("increment-button").getByRole("button").click();
}

export async function waitForGlobal(
  page: Page,
  pred: (n: number) => boolean,
  timeoutMs = 30_000,
) {
  await expect
    .poll(async () => pred(await parseGlobal(page)), { timeout: timeoutMs })
    .toBe(true);
}

/** Idle flush is 1s; wait a bit past that plus network settle. */
export const IDLE_FLUSH_WAIT_MS = 1_200;

export const test = base;
export { expect };
