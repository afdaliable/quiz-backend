# Onboarding — Quiz Backend

Read this once, end to end, before touching the code. It gives you the map;
the existing docs in this repo give you the detail. This document does not
duplicate them — it tells you which one to open for what, and explains the
things no single doc currently connects: how the pieces fit together, why
certain things are built the way they are, and where the sharp edges are.

**Where things live:**

| Question | Read |
|---|---|
| What does endpoint X return / what SQL does it run? | [`docs/api-endpoint-queries.md`](./api-endpoint-queries.md) |
| Full public API reference | [`API_DOCUMENTATION.md`](../API_DOCUMENTATION.md) |
| Full admin API reference | [`ADMIN_API_DOCUMENTATION.md`](../ADMIN_API_DOCUMENTATION.md) + [`ADMIN_TESTING_GUIDE.md`](../ADMIN_TESTING_GUIDE.md) |
| Product-level goals/scope | [`PRD_BACKEND.md`](../PRD_BACKEND.md) |
| How paket_soal ↔ soal mapping works | [`PACKAGE_QUESTIONS_MAPPING.md`](../PACKAGE_QUESTIONS_MAPPING.md) |
| Licensing/premium implementation | [`backend-implementation-guide-license.md`](../backend-implementation-guide-license.md), [`frontend-integration-guide-license.md`](../frontend-integration-guide-license.md) |
| CORS setup | [`CORS-CONFIGURATION.md`](../CORS-CONFIGURATION.md) |
| Live, generated request/response schemas | `/swagger-ui/` on any running instance (see `src/docs.rs`, backed by `utoipa`) |
| Schema history | `migrations/*.sql`, applied in filename/date order — **treat this directory as the source of truth for the current schema**, not `db.sql` (see [Migration strategy](#migration-strategy)) |

> Per repo memory: Swagger coverage is known to be incomplete. When in doubt about what an endpoint actually does, read the controller source or curl the running server — don't trust the Swagger UI or the markdown docs blindly.

---

## 1. Architecture

### 1.1 The four repos

This backend is one of four repos that make up the product:

| Repo | Stack | Role |
|---|---|---|
| **quiz-backend** (this repo) | Rust / Actix-Web / SQLx / MySQL | Single API serving all three clients below, at `https://api.nagih.id` |
| quiz-frontend | Angular | Public-facing site — end users take quizzes here |
| quiz-admin | Next.js | Internal admin panel — content team manages questions/packages/users |
| quiz-mobile-app | Flutter (web-only currently) | Mobile client, same API |

There is no BFF layer and no GraphQL gateway — all three clients hit this
same Actix-Web server directly over REST, distinguished only by which
routes they call and which auth path they use (see §5.1).

Note: this repo (`/mnt/devarea/devarea/rustproject/quiz-backend`) is the
**real development repo**. A separate path
(`MONOREPO_QUIZ/quiz-backend-prod`) exists as a git submodule/mirror inside
the admin monorepo and is reference-only — do not edit it directly.

### 1.2 Request flow

```
Client (frontend/admin/mobile)
        │  HTTPS, Bearer token
        ▼
Actix-Web HttpServer (main.rs)
        │  global middlewares, applied to every request:
        │    1. Cors::default() — permissive: allow_any_origin(), credentials supported
        │    2. AuthMiddleware — Supabase/Google JWT check (see §3.1), route-based bypass list
        ▼
per-scope route matching (web::scope + .service(...) registered in main.rs)
        │  some scopes add a SECOND middleware:
        │    AdminMiddleware — wraps every /admin/* scope, Authentik-first (see §3.1)
        ▼
controller fn (src/controller/*.rs)
        │  extracts path/query/json, calls into...
        ▼
dao (src/dao/*.rs) — thin SQLx query wrappers, one struct per table/domain
        │
        ▼
MySQL/MariaDB `dbquizapp` — either the write pool (`AppState.context`) or
        the read pool (`AppState.read_context`), see §1.3
```

Two side systems controllers may also touch:
- **Redis** (`AppState.redis_pool`, optional — server runs without it if the
  connection fails at boot, `src/main.rs:51-61`) — used for session storage
  and caching static-ish lists (`CKEY_LIST_PAKET_SOAL`, etc. via
  `src/service/redis_service.rs`).
- **AI service** (`AppState.ai_service`, optional — only initialized if a
  top-level `"ai"` key exists in `config.json`) — DeepSeek primary /
  Gemini fallback, used by `ai_controller.rs` for question enrichment.

There are also two background jobs started in `main.rs`: an hourly expired-session
cleanup (`sessions.delete_expired_sessions`) and a weekly difficulty
recalculation (`recalculate_difficulty`, AFD-206) — both plain `tokio::spawn`
loops, no job queue.

### 1.3 Deployment topology (read/write split)

This is the single most important non-obvious architectural fact in this
codebase: **the backend runs as two separate instances**, deployed by the
same CI pipeline (`.github/workflows/ci.yml`) to two different self-hosted
runners:

- **`deploy-nas`** — the master node. Read/write, points at the primary
  MySQL instance (a Synology NAS in this setup).
- **`deploy-pc`** — a second instance. Writes still go to the NAS (master),
  but reads are served from a local MySQL **replica** on that machine.

This is why `AppState` (`src/lib.rs`) carries *two* database handles:

```rust
pub context: Arc<Database<'a>>,       // write pool — always the master
pub read_context: Arc<Database<'a>>,  // read pool — replica if configured, else == context
```

`Config::has_read_replica()` / `get_read_database_url()`
(`src/config.rs:104-115`) check for an optional `dao.read_address` in
`config.json`; if absent, `read_context` is just a clone of the same `Arc`
as `context` (no replica, single pool shared). Controllers deliberately
choose `data.context` for writes and `data.read_context` for reads (see
`soal_controller.rs::get_soal` using `read_context`) — this split has to be
respected when adding new read endpoints, or you defeat the point of
running a replica on the PC node.

Both deployments run the same Docker image
(`ghcr.io/afdaliable/quiz-backend-prod`); `config.json` is bind-mounted in
from the host (`docker run -v .../config.json:/app/config.json`), not baked
into the image. `main.rs` hardcodes the filename `"config.json"` — see §3.4
for how `config.json` vs `config-prod.json` actually gets selected.

### 1.4 Module/controller layout

- `src/main.rs` — process entrypoint: config load, DB/Redis/AI init, all
  route registration, CORS, background jobs.
- `src/controller/*.rs` — one file per resource area, each exposing an
  `init(cfg: &mut web::ServiceConfig)` (or `configure_routes`) that
  registers a `web::scope(...)`. Admin-area controllers additionally
  `.wrap(AdminMiddleware::new())` on their scope.
- `src/dao/*.rs` — one struct per table/domain, holding an `sqlx::Pool` and
  exposing typed query methods. `src/dao/db_context.rs` (`Database` struct)
  aggregates all of them; this is what `AppState.context`/`read_context`
  point at.
- `src/model/*.rs` — request/response DTOs and DB row structs
  (`#[derive(FromRow)]`), plus `utoipa::ToSchema` for OpenAPI generation.
- `src/middleware/` — `auth_middleware.rs` (global JWT gate) and
  `admin_middleware.rs` (per-scope admin gate). See §3.1.
- `src/service/*.rs` — business logic that doesn't belong in a controller:
  payment gateways (Mayar, Midtrans), AI enrichment, CSV import, XP/leveling,
  difficulty recalculation, Redis caching helpers.
- `src/docs.rs` — `utoipa::OpenApi` derive aggregating endpoint docs, served
  at `/swagger-ui/` and `/api-docs/openapi.json`. **Known incomplete** — new
  endpoints don't always get added here; don't treat it as authoritative.
- `src/config.rs` — typed `Config` struct deserialized from `config.json`.

### 1.5 Route registration order matters

`main.rs` registers ~25 controller scopes via `.configure(...)`. Actix-web
matches scopes in **registration order, first match wins**. There's an
explicit comment in `main.rs` (around the `admin_simulasi_controller` /
`admin_hierarchy_controller` registration) documenting a real footgun here:
`admin_hierarchy_controller` registers a catch-all `web::scope("/admin")`
for bulk operations, and if it were registered before more specific
`/admin/...` scopes, it would swallow their paths. If you add a new
`/admin/*` controller, register it **before**
`controller::init_admin_hierarchy_controller` in `main.rs`, or it may
silently never get hit.

---

## 2. Domain model

### 2.1 Core entity map

```
exam_tracks (1) ──< categories (many) ──< subcategories (many) ──< topics (many)
                                                                        │
                                                                        │ (legacy single FK, still present)
                                                                        │
soal (questions) ──────track_id/category_id/subcategory_id/topic_id────┘
   │  │
   │  └──< question_topics (M2M) >── topics        (AFD-226, current way to multi-classify)
   │  └──< question_tags (M2M)   >── tags           (AFD-200, freeform labels)
   │  └── passage_id → passages                     (AFD-224, shared reading-comprehension text)
   │
   └──< paket_soal_items >── paket_soal (quiz packages / "tryout")
                                  │
                                  └── kategori_soal (legacy flat category — NOT the same as
                                                      the exam_tracks/categories taxonomy above)

users ──< user_subscriptions >── premium_plans
users ──< license_codes >── premium_plans          (license-code redemption path)
paket_soal ──< premium_quiz_access >── premium_plans  (per-package min-plan gating)
```

Two things worth flagging explicitly because they're easy to conflate:

- **Two separate "category" systems coexist.** `kategori_soal` /
  `paket_soal.kategori_id` is the original, flat categorization used to
  group *packages* (e.g. "UTBK", "CPNS" — see `db.sql`). The
  `exam_tracks → categories → subcategories → topics` hierarchy (added by
  migration `20260408_afd200_taxonomy_hierarchy.sql`, AFD-200) is a newer,
  deeper taxonomy attached directly to *questions* (`soal.track_id`,
  `category_id`, `subcategory_id`, `topic_id`). They are not the same
  table and not FK-linked to each other. Don't assume "category" means the
  same thing across controllers — check which table is actually joined.

- **`soal.topic_id` vs `question_topics`.** `soal` still carries a single
  `topic_id` FK column (added in AFD-201). Migration
  `20260414_afd226_question_topics_m2m.sql` (AFD-226) added the
  `question_topics` pivot table (question ↔ topic, many-to-many, mirroring
  the pre-existing `question_tags` pattern) and backfilled it from the
  existing single FK (`INSERT IGNORE INTO question_topics ... SELECT id,
  topic_id FROM soal WHERE topic_id IS NOT NULL`). The single-FK column
  was **not** dropped. In current code (`admin_soal_controller.rs`), a
  question update accepts an optional `topic_ids` array and calls
  `taxonomy_dao.set_question_topics(...)` to manage the M2M table
  independently of the legacy `topic_id` scalar field, which is still
  updatable via its own `COALESCE(?, topic_id)` clause. Reads
  (`get_question_by_id`) populate `question.topic_ids` from the M2M table.
  Net effect: a question's "topic" can currently be represented in two
  places that are not automatically kept in sync by a trigger or
  constraint — application code is responsible for consistency.

### 2.2 Key entities

| Entity | Table | Notes |
|---|---|---|
| Question | `soal` | Core content unit. `question_type` discriminates `multiple_choice` / `true_false` / `fill_blank`. `status` (`draft`/`active`/`archived`), `format`, `bloom_level`, `difficulty_est`/`difficulty_calc` added by AFD-201 for the enrichment/analytics pipeline. `passage_id` optional FK for reading-comprehension sets. |
| Passage | `passages` | Shared text block multiple `soal` rows can reference (AFD-224). `ON DELETE SET NULL` on `soal.passage_id` — deleting a passage does not delete its questions. |
| Package / "tryout" | `paket_soal` | A named bundle of questions (`paket_soal_items` join table to `soal`). `is_premium` flag gates the whole package. |
| Taxonomy hierarchy | `exam_tracks`, `categories`, `subcategories`, `topics` | 4-level tree, each level FK'd to its parent with `ON DELETE CASCADE`. UUID (`CHAR(36)`) primary keys, unlike `soal`'s integer PK. |
| Tags | `tags`, `question_tags` | Freeform M2M labels on questions, independent of the taxonomy tree. |
| User | `users` | `id` is `CHAR(36)` (UUID), `provider` distinguishes `local` vs OAuth signups, soft-deleted via `deleted_at`. |
| Premium plan | `premium_plans` | Plan catalog (price, duration, `is_lifetime`, Mayar product linkage). |
| Subscription | `user_subscriptions` | User ↔ plan, time-bounded (`start_date`/`end_date`), `status` string. |
| License code | `license_codes` | Alternate premium-access path: a redeemable code tied to a plan and (optionally) a Mayar transaction. See `backend-implementation-guide-license.md`. |
| Premium quiz access | `premium_quiz_access` | Per-package override: a `paket_soal` can require a specific `min_plan_id`, independent of the user's general subscription tier. |
| Quiz session / attempt | `quiz_sessions`, `quiz_answers`, `jawaban_user` | Session-scoped attempt tracking; `jawaban_user` looks like an older/parallel answer-tracking table to `quiz_answers` — confirm which is currently written to before relying on either (not verified in this pass; check `src/dao/quiz_session_dao.rs` and `src/controller/quiz_session_controller.rs`). |

> `db.sql` (checked into repo root) is a **stale snapshot** — it predates
> the entire taxonomy hierarchy, `passages`, and several `soal` columns
> (`track_id`, `status`, etc.). Do not use it as the schema reference;
> use it only as a rough starting point and apply `migrations/*.sql` on
> top mentally (or against a real DB) to get the current shape. See
> §3.2 for why the migration directory itself isn't a clean linear history
> either.

---

## 3. Key decisions

### 3.1 Auth strategy: two independent middlewares, two different mechanisms

There are **two separate, independently-implemented auth layers**, not one
shared abstraction:

1. **`AuthMiddleware`** (`src/middleware/auth_middleware.rs`) — wraps the
   *entire app* (`.wrap(AuthMiddleware::new(jwt_secret))` in `main.rs`).
   Runs on every request. Tries to decode the Bearer token first as a
   Google OAuth JWT, then as a Supabase JWT (same HMAC secret,
   `config.jwt_secret`, used for both — they're distinguished by claim
   shape, not by a different key). A hardcoded `PUBLIC_ROUTES` allowlist
   (27 path prefixes, `auth_middleware.rs:98-126`) bypasses this check
   entirely — notably `/admin`, `/ai`, and `/internal` are in that list,
   meaning **this middleware does not protect admin routes at all**; it
   explicitly defers to whatever runs next.

2. **`AdminMiddleware`** (`src/middleware/admin_middleware.rs`) — applied
   per-scope, only on `/admin/*` (and `/ai/*`) route groups, via
   `.wrap(AdminMiddleware::new())` inside each admin controller's `init()`.
   This is where the real admin auth happens, in two steps:
   - **Primary: Authentik OIDC.** The bearer token is sent to Authentik's
     `/application/o/userinfo/` endpoint (URL overridable via
     `AUTHENTIK_USERINFO_URL` env var). If valid, the response's `groups`
     claim is checked for membership in `quiz-admins` (overridable via
     `AUTHENTIK_ADMIN_GROUP`). This is a live network call per admin
     request — no local JWT validation for this path.
   - **Fallback: legacy HMAC JWT.** If Authentik validation fails (token
     isn't an Authentik token, network error, etc.), it falls back to
     decoding the token as a local HS256 JWT signed with the *same*
     `config.jwt_secret` used by `AuthMiddleware`/`admin_login`
     (`auth_controller.rs::admin_login`, `POST /auth/login`), then does a
     **live DB lookup** (`SELECT role FROM users WHERE id = ?`) to check
     `role IN ('admin', 'superadmin')`.

   This two-tier design is clearly intentional (the fallback is explicitly
   commented "legacy") — Authentik was introduced later as the primary
   admin IdP, and the original DB-role-based JWT login was kept working
   rather than ripped out, presumably to avoid a hard cutover for existing
   admin sessions/tooling. If you're adding new admin-protected routes, you
   get both paths for free by using `AdminMiddleware`; you don't need to
   reimplement either check.

   Public-facing user auth (`AuthMiddleware`) is unrelated to this and
   still just Supabase/Google JWT — there is no Authentik involvement on
   the consumer side.

### 3.2 COALESCE-on-update pattern (and where it's broken)

Nearly every admin `PUT`/`PATCH` update endpoint in this codebase follows
the same pattern to support **partial updates**: the SQL `UPDATE` sets
every column to `COALESCE(?, column)`, and the bound parameter is the
`Option<T>` from the request DTO. If the client omits a field (sends `null`
or leaves it out of the JSON body), `COALESCE` falls back to the existing
DB value instead of overwriting it with `NULL`. This pattern appears in
`admin_soal_controller.rs`, `admin_kategori_controller.rs`,
`admin_packages_controller.rs`, `admin_user_controller.rs`,
`admin_analytics_controller.rs`, `bookmark_controller.rs`, and
`ai_controller.rs` — it's a deliberate, repo-wide convention, not a
one-off.

**This convention is broken for one field.** See §6 (Known gotchas) —
`passage_id` in `PUT /admin/soal/{id}` is bound directly, without
`COALESCE`, unlike every other column in that same query.

### 3.3 Migration strategy

`migrations/*.sql` is **not** managed by a migration framework (no
`sqlx migrate`/`diesel migrate` tooling detected — no `_sqlx_migrations`
bookkeeping table reference in code, no migration runner invoked from
`main.rs`). Filenames follow a loose, evolving convention:

- Oldest files: purely descriptive names (`add_columns_to_soal.sql`,
  `create_bookmarked_questions.sql`) — no date, no ticket ID.
- Later files: `YYYYMMDD_description.sql` (e.g.
  `20250811_create_quiz_sessions_table.sql`).
- Most recent files: `YYYYMMDD_afdNNN_description.sql`, where `afdNNN` is a
  ticket/task reference (e.g. `20260408_afd200_taxonomy_hierarchy.sql`).
  Comments inside these files usually explain the *why* (see
  `20260410_afd224_passages.sql`'s comment: "Safe for production:
  passage_id NULL = soal biasa, semua data existing tetap valid").

Since there's no runner, **applying a new migration is a manual step** —
presumably run by hand against the master (NAS) DB, with the replica
picking it up via MySQL replication. When adding a migration: follow the
current `YYYYMMDD_afdNNN_description.sql` naming, write it idempotently
where practical (`CREATE TABLE IF NOT EXISTS`, `INSERT IGNORE`), and add a
comment stating what it does and why, matching the existing recent files.
Don't assume `db.sql` needs to be kept in sync — it doesn't appear to be
regenerated as part of this process (see §2.2 staleness note).

### 3.4 Config split: `config.json` vs `config-prod.json`

Both files are **git-ignored** (`.gitignore: /config.json`,
`/config-prod.json`) — they hold live secrets (DB passwords, JWT secret,
OAuth client secrets, payment gateway keys) and are not meant to be
committed. `Config::from_file` (`src/config.rs:86-89`) is hardcoded to
read literally `"config.json"` — there's no env-var-based file selection
in the Rust code itself.

The dev/prod split happens entirely **outside** the binary, at deploy
time: `.github/workflows/ci.yml` builds one Docker image and bind-mounts a
host-side `config.json` into the container
(`-v /volume1/docker/quiz/backend/app/config.json:/app/config.json` on the
NAS runner, a different host path on the PC runner). `config-prod.json` in
this repo is effectively a *template/reference* for what gets deployed as
`config.json` on each host — the two files differ in DB host
(`10.100.44.124` local vs `100.87.162.99`/NAS), `midtrans.is_production`,
`google_oauth.redirect_uri`, and `upload_dir`. There is no code path that
reads `config-prod.json` directly; if you're editing config for a
non-local environment, you're editing the file that gets bind-mounted on
that host, not this repo's copy.

---

## 4. Validation rules

### 4.1 Enforced at the DB level

- **NOT NULL**: e.g. `soal.soal` (question text), `users.email`,
  `users.display_name`, `paket_soal.nama_paket_soal` — see `db.sql` /
  relevant `CREATE TABLE`/`ALTER TABLE` in `migrations/`.
- **UNIQUE constraints**: `users.email`, `kategori_soal.nama_kategori`,
  taxonomy slugs are unique *per parent* (e.g.
  `idx_categories_track_slug (track_id, slug)` — same slug can exist under
  different tracks, not globally).
- **Foreign keys with explicit `ON DELETE` behavior** — and the behavior
  differs meaningfully by relationship, which is itself a modeling
  decision worth knowing:
  - `categories → exam_tracks`, `subcategories → categories`,
    `topics → subcategories`, `question_tags/question_topics → soal`:
    `ON DELETE CASCADE` — deleting a parent taxonomy node or a question
    wipes its children/associations.
  - `soal.track_id/category_id/subcategory_id/topic_id →` taxonomy tables,
    `soal.passage_id → passages`: `ON DELETE SET NULL` — deleting a
    taxonomy node or a passage does **not** delete affected questions, it
    just orphans the reference. This is why `soal` fields like `topic_id`
    are nullable even though a question is expected to have a topic in
    normal operation.
  - `paket_soal_items → paket_soal`, `paket_soal_items → soal`,
    `paket_soal → kategori_soal`: `ON DELETE CASCADE`.
- **ENUM columns** constrain values at the DB layer without app-level enum
  validation needed: `soal.correct_answer`, `soal.question_type`,
  `soal.difficulty_est`/`difficulty_calc`, `soal.bloom_level`,
  `soal.format`, `soal.status`, `exam_tracks.status`,
  `license_codes.status`.

### 4.2 Enforced at the app level

- **Auth/authorization** — entirely app-level, see §3.1. No DB-level row
  security; every controller trusts the middleware to have already gated
  access and to have set the `user_id` header used downstream
  (`AuthenticatedUser::from_request` in `auth_middleware.rs` just reads
  that header — it does **not** re-verify the token itself).
- **Business-rule checks in controllers**, e.g. `update_question` in
  `admin_soal_controller.rs` rejects an empty/whitespace-only `soal` text
  before hitting the DB (`admin_soal_controller.rs:384-390`), even though
  the DB only enforces `NOT NULL`, not non-blank.
  Non-empty JSON payloads that omit a field rely entirely on the
  `COALESCE` pattern (§3.2) rather than explicit "was this field
  provided" checks.
- **Premium/license gating** is app-level logic layered on top of plain
  relational data — `premium_quiz_access.min_plan_id`,
  `user_subscriptions.status`/`end_date`, and `license_codes.status` are
  just data; the actual "can this user access this package" decision is
  computed in controller/service code (see `premium_controller.rs`,
  `license_controller.rs`, `check-quiz-access` endpoint in
  `soal_controller.rs`), not enforced by a DB constraint or view.
- **Internal routes** (`/internal/*`) are gated by IP allowlist + a static
  API key rather than user auth — see the "Internal routes" comment in
  `auth_middleware.rs:118-119` and `internal_soal_controller.rs`.

---

## 5. Core flows

### 5.1 Auth flow: login → token → protected route

**Public/consumer side** (frontend, mobile — Supabase/Google):
1. Client authenticates against Supabase directly (or completes Google
   OAuth via `POST /auth/google/callback`, `auth_controller.rs:252`),
   receiving a JWT.
2. Client sends that JWT as `Authorization: Bearer <token>` on every
   subsequent request.
3. `AuthMiddleware` (global) intercepts, checks the path against
   `PUBLIC_ROUTES`; if not public, decodes the token — Google claims shape
   first, then Supabase claims shape — using `config.jwt_secret` as the
   HS256 key for both. On success it injects an `x-user-id`-equivalent
   header (`user_id`) into the request and lets it through; on failure
   (expired, invalid, missing) it short-circuits with a 401 JSON body
   before the request reaches any controller.
4. Downstream controllers/extractors that need the caller's identity use
   `AuthenticatedUser` (`FromRequest` impl,
   `auth_middleware.rs:52-85`), which just reads the `user_id` header the
   middleware set — it trusts the middleware completely, does no
   re-validation.

**Admin side** (quiz-admin panel):
1. Two possible entry points: Authentik login (external IdP, out of this
   repo's scope) yielding an Authentik-issued token, **or** the legacy
   local path — `POST /auth/login` (`auth_controller.rs:109`) — which
   looks up the user by email in the local `users` table, checks
   `role IN ('admin','superadmin')`, and (notably) **accepts any password**
   for admin users (`admin_soal_controller.rs` comment at
   `auth_controller.rs:131-132`: "For testing, we'll accept any password
   for admin users... In production, you should verify against a hash" —
   this has evidently not been revisited; treat `/auth/login` as
   unauthenticated-by-password in its current state). On success it mints
   an HS256 JWT with `config.jwt_secret` and also creates a DB session row
   (`sessions` table / `create_session_with_redis`, cached in Redis if
   available).
2. Client calls `/admin/*` with `Authorization: Bearer <token>` (either
   token type).
3. `AdminMiddleware` (per-scope) runs — Authentik validation first, legacy
   HMAC-JWT-plus-DB-role-check fallback second (§3.1). Only on success does
   the request reach the controller.

### 5.2 Write flow: `PUT /admin/soal/{id}` — admin editing a question

1. Request hits `/admin/soal/{id}` → `AuthMiddleware` sees the path starts
   with `/admin`, treats it as public (no-op) → falls through to the
   `/admin/soal` scope's own `AdminMiddleware` (Authentik or legacy JWT +
   role check, §3.1).
2. `update_question` (`admin_soal_controller.rs:372`) deserializes the body
   into `UpdateSoalRequest` — every field is `Option<T>`, so the client can
   send just the fields it's changing.
3. One app-level check: if `soal` (question text) was provided, reject if
   it's blank after trimming.
4. Runs the big `COALESCE`-pattern `UPDATE` (§3.2) against `data.context`
   (the **write** pool — not `read_context`).
5. If `tag_ids` was provided, syncs `question_tags` via
   `TaxonomyDao::set_question_tags`. If `topic_ids` was provided (AFD-226),
   syncs `question_topics` via `TaxonomyDao::set_question_topics`
   independently of the scalar `topic_id` column updated in step 4 (see
   §2.1 for why these can drift).
6. Re-fetches the full row via `data.context.soal.get_soal_by_id` (note:
   uses the write pool for the re-read too, not `read_context` — avoids a
   replication-lag read-your-writes issue on the PC deployment), populates
   `topic_ids` on the response from the M2M table, returns it as JSON.

See §6 for the `passage_id` bug embedded in step 4.

### 5.3 Public read flow: `GET /soal/{id}`

1. Request hits `/soal/{id}`. This path is **not** in `PUBLIC_ROUTES`
   (only `/soal/search` is), so `AuthMiddleware` requires a valid
   Supabase/Google JWT — despite the "public read" framing, this
   particular endpoint is authenticated. (`GET /soal/search`, by contrast,
   genuinely allows anonymous access.)
2. `get_soal` (`soal_controller.rs:34`) calls
   `app_state.read_context.soal.get_soal_by_id(...)` — deliberately the
   **read** pool, so on the PC deployment this is served from the local
   replica rather than round-tripping to the NAS.
3. Returns the row as JSON, or 404 if not found (errors are collapsed to a
   bare 404 — the underlying DB error is not surfaced to the client).

A genuinely anonymous listing flow is `GET /listpaketsoal` /
`GET /paket-soal-response/{nama_kategori}` — these are also gated by
`AuthMiddleware` the same way (not in the public list) except where noted;
check `PUBLIC_ROUTES` directly before assuming any given `GET` is open.

---

## 6. Known gotchas / bugs

### 6.1 `passage_id` is not COALESCE'd in `PUT /admin/soal/{id}` (confirmed live in this pass)

File: `src/controller/admin_soal_controller.rs`, inside `update_question`,
the `UPDATE dbquizapp.soal SET ...` statement starting at line 393.

```rust
// line 393
let result = sqlx::query(
    r#"
    UPDATE dbquizapp.soal
    SET passage_id = ?,                          // line 396 — NOT wrapped in COALESCE
        soal = COALESCE(?, soal),                // every other field below IS
        question_type = COALESCE(?, question_type),
        ...
```

Every other field in this statement follows the repo-wide COALESCE
convention (§3.2): if omitted from the request body, the existing DB value
is preserved. `passage_id` does not — it's bound directly
(`.bind(question_req.passage_id)` at line 424, where
`question_req.passage_id: Option<i32>`). Since `UpdateSoalRequest`
deserializes missing/`null` JSON fields to `None`, and `sqlx` binds `None`
as SQL `NULL`:

**Any admin PUT to `/admin/soal/{id}` that doesn't explicitly include
`passage_id` in its JSON body will silently NULL out that question's
passage association — even if the client only intended to change, say,
`solution` text.**

This is a real footgun for any admin-panel form that doesn't round-trip
`passage_id` on every save (e.g. a "quick edit solution" modal that only
serializes the fields it shows). It's also silent — no error, no warning,
the response just comes back with `passage_id: null` and the caller may
not notice unless they specifically check.

**If you're touching this function:** wrap it in `COALESCE(?, passage_id)`
to match every other field, unless there's a deliberate reason found
elsewhere (e.g. a separate "detach from passage" endpoint that's supposed
to null it — none was found in this pass, `admin_soal_controller.rs` has
no dedicated passage-detach route) for it to behave differently. Confirm
with whoever owns `quiz-admin`'s question-edit UI whether it currently
always sends `passage_id`, since that determines whether this bug is
currently live in production or dormant.

### 6.2 `soal.topic_id` (scalar) and `question_topics` (M2M) can drift

Covered in §2.1. The AFD-226 migration didn't drop the legacy `topic_id`
column, and `update_question` updates both independently based on whatever
the request happened to include (`topic_id` via COALESCE, `topic_ids` via
a separate M2M sync only if present). A client that sends `topic_ids` but
not `topic_id` (or vice versa) can leave the two representations
disagreeing about a question's topic.

### 6.3 Admin login accepts any password

`POST /auth/login` (`auth_controller.rs:109-237`) checks the user exists
and has an admin role, but does not check any password/credential at all
— see the comment at `auth_controller.rs:131-132`. This is the *fallback*
credential path (Authentik is primary for the admin panel per §3.1), but
it's still live and reachable. Worth confirming with the team whether this
route is still exposed to `quiz-admin` in practice or effectively dead
since Authentik rollout.

### 6.4 `PUBLIC_ROUTES` matching is prefix-based, not exact

`auth_middleware.rs:173-175` treats any of the 27 entries as a prefix
match (with a boundary check so `/soal` doesn't accidentally match
`/soalfoo`, but a literal `/admin` entry does mean **every** path under
`/admin/...` bypasses `AuthMiddleware` entirely, relying wholly on
`AdminMiddleware` for protection). If a future controller registers a
route that happens to share a prefix with one of these 27 entries but
isn't meant to be public, it will be silently exempted from the global
JWT check. Read the full list before adding new top-level route prefixes.

### 6.5 `config.json`/`config-prod.json` contain live secrets on disk

Not a code bug, but worth knowing: both files exist un-redacted in the
working tree of this dev repo (they're git-ignored, so not committed, but
they are real, working credentials for the dev DB, Supabase, Google OAuth,
Mayar, etc.). Be careful with tooling that globs `*.json` or otherwise
might expose these — e.g. don't paste their contents into shared docs or
tickets.

---

## 7. How to extend this doc

Keep this file a map, not an encyclopedia — when you learn something that
changes how a newcomer should understand the *shape* of the system (a new
service boundary, a new table relationship, a new auth mechanism, a new
enforced-vs-app-level validation rule, or a new core flow), add or update
the relevant section here in the same change that introduces it, rather
than only updating the narrower doc (`API_DOCUMENTATION.md`,
`ADMIN_API_DOCUMENTATION.md`, etc.) that covers just the mechanical
request/response shape. If you find a claim in here that's gone stale,
correct it in place — don't leave a note "TODO: verify" without also
re-verifying it against the current code, since that's exactly the kind of
guess this doc is meant to avoid.
