# Web Admin UI

## Goal

Add a Bun-driven operator web interface to `proxy-broker` without changing the
existing JSON API contracts. The first release must ship as a single Rust
binary that serves the built SPA on the same origin as the API, while keeping a
separate Vite and Storybook workflow for local development.

## Runtime Shape

- Backend remains the source of truth for subscription loading, refresh, IP
  extraction, node inventory, and session lifecycle.
- Frontend lives in `web/` and is built with `Bun + Vite + React + TypeScript`.
- Production serving model:
  - `GET /` returns the SPA shell.
  - `GET /assets/*` serves bundled frontend assets.
  - Non-API frontend routes fall back to `index.html`.
  - `/api/v1/*` and `/healthz` keep their current behavior and priority.
- Storybook is development/documentation only and is not served by the
  production binary.

## Frontend Stack

- Package/runtime: Bun 1.x
- App framework: React 19 + React Router
- Styling: Tailwind CSS 4 + CSS variables
- Component system: shadcn/ui
- Data/query layer: TanStack Query
- Form/validation: React Hook Form + Zod
- Lint/format: Biome
- Unit/component tests: Vitest + Testing Library
- Component docs and interaction tests: Storybook 10 +
  `@storybook/test-runner`
- E2E smoke: Playwright

## Information Architecture

### Routes

- `/`
  - service health card
  - profile selector
  - subscription load form (`url` or server-side file path)
  - refresh card and latest refresh summary
- `/nodes`
  - server-driven node filters, sorting, and pagination
  - view toggle for flat, IP-grouped, region-grouped, and subscription-grouped presentations
  - batch export and batch session creation actions
- `/ips`
  - compatibility route that redirects to `/nodes`
- `/sessions`
  - single open form
  - batch open form
  - active sessions table with close action

### Persistence

- The browser stores only UI-local preferences:
  - last used `profile_id`
  - last selected source type
  - last used nodes workspace filters/view mode if implemented as convenience state
- No client-side authoritative data cache beyond TanStack Query.
- URL subscription downloads remain server-side and use a compatibility UA
  fallback set (`Clash.Meta/1.18.3`, `mihomo/1.18.3`, `Clash Verge/1.7.7`);
  the UI does not expose a custom UA field.

## Component Boundaries

- Route containers are thin:
  - wire queries and mutations
  - map API results into view props
  - own route-level loading and error surfaces
- Shared UI components are props-driven and Storybook-friendly:
  - metric cards
  - API state banners
  - filter chips / badges
  - tables and grouped list sections
  - form sections and field groups
- shadcn primitives added to the repo are treated as first-class components and
  must be documented like custom components.

## Localization

- The operator UI ships with two supported locales:
  - `en-US` as the baseline catalog
  - `zh-CN` for Simplified Chinese (Mainland China)
- Locale resolution order is fixed:
  - persisted browser preference from `proxy-broker.locale`
  - browser language detection (`zh*` resolves to `zh-CN`)
  - fallback to `en-US`
- The active locale must update:
  - the app translation context
  - `document.documentElement.lang` using `zh-Hans` for `zh-CN` and `en` for `en-US`
  - locale-aware date/time and number formatting
- The language switcher lives in the application shell footer beside the theme
  control and must switch immediately without a full page reload.
- Known backend error codes, task kinds, task stages, task statuses, and task
  triggers are localized in the frontend. Unknown backend free-text messages
  stay visible as raw strings rather than being hidden or replaced.

## Storybook Contract

- Story files are colocated with each component or page as `*.stories.tsx`.
- Every component or page under these paths must have a story file:
  - `web/src/components/ui/**`
  - `web/src/components/**`
  - `web/src/features/**/components/**`
  - `web/src/layouts/**`
  - `web/src/pages/**`
- Exclusions:
  - hooks
  - query helpers
  - fixtures
  - providers
  - type-only files
  - utility modules without UI
- Every story file must:
  - opt into autodocs with `tags: ["autodocs"]`
  - declare `title`
  - declare `component`
  - provide `parameters.docs.description.component`
- Minimum story coverage:
  - `Default` for every component/page
  - additional named stories for any supported public state such as `loading`,
    `empty`, `error`, `disabled`, `open`, or `populated`
- Storybook preview must inject:
  - app styles
  - router context
  - query client
  - toast/provider context
  - a stable theme shell
  - locale context with toolbar controls for `theme` and `locale`
- A Bun verification script must fail CI when a covered component/page has no
  story or when a story file is missing autodocs metadata.
- App shell and route-level page stories must expose stable `zh-CN` variants or
  explicit `globals.locale = "zh-CN"` presets so screenshots are reproducible.

## Visual Direction

- Light-first admin console, tuned for dense operational work.
- Palette:
  - warm neutral background
  - slate text
  - teal/blue accents for primary actions and status highlights
- Typography:
  - expressive but readable headline font
  - monospace treatment for ports, IPs, IDs, and file paths
- Layout:
  - desktop-first split panels and data cards
  - mobile collapses into stacked sections with no hidden critical actions

## Visual Evidence

### App Shell Locale Controls

![App shell locale controls](assets/web-admin-ui/appshell-zh-cn.png)

- Storybook source: `Components/AppShell > ZhCN`
- Confirms the footer language switcher sits beside the theme control and the
  surrounding chrome localizes to `zh-CN`.

### Overview Page (`zh-CN`)

![Overview page zh-CN](assets/web-admin-ui/overviewpage-zh-cn.png)

- Storybook source: `Pages/OverviewPage > ZhCN`
- Confirms the overview shell, workflow guidance, forms, alerts, and access
  control surfaces render in Simplified Chinese.

### Tasks Page (`zh-CN`)

![Tasks page zh-CN](assets/web-admin-ui/taskspage-zh-cn.png)

- Session orchestration view with the simplified three-mode open flow, advanced
  exclusions, and the live session table used for close actions.

## Build and Tooling

### Frontend Scripts

- `bun run dev` binds `127.0.0.1:38181`
- `bun run build` builds `web/dist`
- `bun run preview` previews the app locally
- `bun run check` runs Biome checks
- `bun run test` runs Vitest
- `bun run verify:stories` enforces story coverage and autodocs metadata
- `bun run storybook` binds `127.0.0.1:38182`
- `bun run build-storybook` outputs the static Storybook site
- `bun run test-storybook` runs Storybook interaction coverage
- `bun run test:e2e` runs the browser smoke flow

### Backend Build Rules

- `cargo build --release` must fail with a clear message if
  `web/dist/index.html` is missing.
- Runtime serving should prefer embedded assets for release builds.
- API handlers should keep the existing workspace boundaries while allowing the
  Nodes workspace to add node query/export/open-session routes and the Sessions
  workspace to keep its suggested-port and searchable option helper routes.

## Test Matrix

- Rust:
  - existing unit tests remain green
  - add route tests for `/`, `/assets/*`, SPA fallback, `/api/v1/*`,
    `/healthz`
- Frontend:
  - unit tests for helpers and small components
  - Storybook interaction tests for component behaviors
  - Playwright smoke for the end-to-end operator flow
- CI:
  - install frontend deps with Bun
  - run frontend checks/tests/docs gates before Rust checks
  - upload `storybook-static` as a PR artifact

## Acceptance Criteria

- A local operator can use the browser UI to:
  - load a subscription from URL or server-side file path
  - refresh profile metadata
  - inspect subscription nodes with server-driven filters, sorting, pagination,
    grouping, export, and batch session actions
  - open single or batch sessions
  - list and close sessions
- The built SPA is reachable from the Rust server root in production.
- URL subscription loads work with upstream providers that gate Clash/Mihomo
  payloads on the request UA, without changing the UI payload shape.
- The UI chooses between `en-US` and `zh-CN` using persisted preference first,
  then browser language detection, and keeps the selected locale stable across
  reloads and route changes.
- The app shell exposes a language switcher, keeps `<html lang>` in sync with
  the active locale's script metadata (`zh-Hans` for `zh-CN`, `en` for
  `en-US`), and localizes known backend error/task enums without changing API
  contracts.
- The Nodes workspace uses backend-driven node query/export/open-session
  endpoints while preserving `/ips` as a compatibility redirect to `/nodes`.
- The Sessions workspace uses a flattened open/open-batch payload plus helper
  endpoints for suggested ports and searchable target options.
- Every committed UI component/page in scope has Storybook docs and stories.
- CI rejects missing stories or missing autodocs metadata.
- The repo remains Bun-first for frontend workflows and Cargo-first for backend
  workflows.
