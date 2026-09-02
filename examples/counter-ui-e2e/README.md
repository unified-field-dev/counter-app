# counter-ui-e2e

Leptos lab host that mounts counter-app pages for Playwright. Lab-only: insecure
session cookie `uf_counter_ui_e2e`, `POST /api/test/seed-data`, harness auth (no
lepton sign-in). Photon WS is mounted for live refetch.

Seed path naming matches **tag-ui-e2e** (`/api/test/seed-data`), not a product-only
alias.

## Run

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cd examples/counter-ui-e2e/end2end && npm ci && npx playwright install chromium
cd ../../..
cargo leptos end-to-end --project counter-ui-e2e
```

Host listens on `127.0.0.1:3000`. Do not Ctrl-C; wait for Playwright to exit.

`public/` ships Orbital boot assets (`orbital-theme-baseline.css`, `main.css`) copied
into `site/` at build time — required for hydrate / boot overlay (same pattern as
chronon-uf-app-e2e).

The lab host mounts the same page components as `CounterRoutes`, without `Lazy`
(wasm-split Lazy under `ParentRoute` panics on hydrate in the current Leptos pin).
Production hosts keep `CounterRoutes` + `--split` when needed.

Anon rate-limit budget defaults to `COUNTER_ANON_INCREMENTS_PER_MIN=3` on this
host (override via env). Seed always clears buckets unless `reset_rate_limit: false`.

## Seed

`POST /api/test/seed-data` with JSON:

```json
{
  "auth": "anonymous" | "owner" | "member" | "unverified",
  "seed_scores": false,
  "reset_rate_limit": true
}
```

Returns `{ ok, auth, fixtures: { global_value?, scores? } }`.

Gauge `CounterAdmin` is synced at host boot, granted to the `counter_admin`
permission group (`counter_app::permissions::COUNTER_ADMIN_GROUP_ID`), and the
`owner` seed user is added as a group member. `member` (`alice`) is verified but
lacks the permission (permission sad path).

## Scenario catalog

| Spec | Scenario ID | Intent |
|------|-------------|--------|
| live | `pw-counter-lab-boot-happy` | `/counter` shows container + global |
| live | `pw-counter-live-anon-incr-happy` | click once; idle flush +1 |
| live | `pw-counter-live-validation-sad` | skip — validation is server-covered (no invalid UI amount) |
| live | `pw-counter-anon-rate-limit-sad` | flush until rate-limit MessageBar |
| batch | `pw-counter-live-batch-idle-happy` | 5 rapid clicks → +5 after idle |
| batch | `pw-counter-live-batch-max-age-happy` | flush by max age (~5s) |
| auth_admin | `pw-counter-user-incr-happy` | owner personal + global bump |
| auth_admin | `pw-counter-admin-gate-sad` | anonymous gated |
| auth_admin | `pw-counter-admin-gate-unverified-sad` | unverified gated |
| auth_admin | `pw-counter-admin-perm-sad` | verified member without CounterAdmin |
| auth_admin | `pw-counter-admin-set-happy` | owner with CounterAdmin sets exact global via admin UI |
| floaters | `pw-counter-floater-spawn-happy` | `+N` floater after flush |
| floaters | `pw-counter-floater-variance-happy` | floater origins/rotation differ |
| photon | `pw-counter-photon-refetch-happy` | second context updates via Photon |
| leaderboard | `pw-counter-leaderboard-page-happy` | seeded scores ordered on page |

These are validating Playwright scenarios for the counter product surface. Label
`product_surface` rows elsewhere as smoke — not these.
