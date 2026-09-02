# Security Policy

## Supported versions

Security fixes are accepted against the latest published `0.1.x` release line of this repository's crates (`counter-app`, `counter-app-worker`, and related example crates).

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Prefer one of the following:

1. **GitHub Security Advisories** — use [Report a vulnerability](https://github.com/unified-field-dev/counter-app/security/advisories/new) on this repository when available.
2. Contact the maintainers privately via the repository owner listed at https://github.com/unified-field-dev/counter-app.

Include:

- a description of the issue and its impact
- steps to reproduce or a proof of concept when possible
- affected crate names and versions

We will acknowledge receipt as soon as practical and coordinate a fix and disclosure timeline with you.

## Scope

In scope: vulnerabilities in this repository's published crates and documentation that could cause unsafe production defaults, plus CI/supply-chain issues in this repository.

Out of scope: vulnerabilities solely in third-party dependencies unless this project mishandles them in a security-relevant way.

## Demo security posture

Counter App is a **demonstration product**. Public increments stay open for teaching;
absolute set and admin UI require Gauge `CounterAdmin`. Do not deploy demo defaults
unchanged to production.

| Surface | Posture |
|---------|---------|
| `SetCounter` / `counter_set` | Requires Gauge `CounterAdmin` via `#[uf_product_macros::server(permission = "CounterAdmin")]`. Session Valence performs the write (no System elevation). Hosts must wire `PermissionBackend`, sync manifests, grant `CounterAdmin` to the `counter_admin` permission group, and add operators as group members. |
| `/counter/admin` | Verified email **and** `CounterAdmin` via `uf_product::routes::RequireAuthenticated` (server fn remains authoritative). Same group membership as set. |
| `counter_increment` / anon global bump | Public demo write. Session or anonymous Valence; `Counter` update policy is `PUBLIC_READ`. |
| Anonymous increment | Per-request cap (`MAX_ANON_INCREMENT_AMOUNT`) plus in-process rate limit (`COUNTER_ANON_INCREMENTS_PER_MIN`, default 60/min). `0` fails closed (denies all). |
| Personal `UserCounter` writes | Session-bound server fns + service actor check + Valence `OWNER_BY_USER_FIELD` / `SYSTEM_ONLY` on create and update. |
| Leaderboard | Resolves display names via viewer Valence; never exposes email; falls back to redacted labels. Page `limit` clamped to `MAX_HIGH_SCORES_LIMIT`. |
| `/ws/counter` | Unauthenticated WebSocket (CA-07 accepted for demo). Payload is the public counter integer only — no user data. Hosts requiring auth should wire `photon-axum` (`photon-leptos`) with a session extractor. |
| Counter Valence update policy | `PUBLIC_READ` at the schema layer (intentional demo default so increments stay open). Increment and set share one Valence `update` op — server-fn authz gates set; direct Valence clients can still mutate under `PUBLIC_READ` until a future set/increment split. |
