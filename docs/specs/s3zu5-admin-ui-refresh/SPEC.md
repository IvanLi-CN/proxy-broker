# Admin UI Refresh

## Goal

Refactor the Bun + React operator console into a denser control-room interface that
keeps the three workflows (load/refresh, IP extraction, session orchestration)
faster to scan and safer to operate, while upgrading Profile handling into a
project workspace model with explicit selection, summary loading, and per-profile
UI state persistence.

## Scope

- Rebuild the shared web admin visual system around a light-first shadcn/ui
  dashboard language with stronger hierarchy, compact data surfaces, and clearer
  keyboard/mouse feedback.
- Replace the freeform Profile text field with an explicit project picker that
  combines known backend profiles, recent browser-local workspaces, and
  confirmed creation of a new project ID.
- Add backend read APIs for project discovery and summary hydration:
  `GET /api/v1/profiles` and `GET /api/v1/profiles/{profile_id}/summary`.
- Persist each project's workspace state in browser localStorage so the overview,
  extraction, and session forms/results restore when switching back.
- Rework the main shell plus `/`, `/ips`, and `/sessions` into a control-room
  layout with route heroes, summary rails, denser tables, and explicit state
  panels.
- Update stories, tests, and smoke coverage for the refreshed UI states.

## Non-Goals

- No server-side persistence of per-page workspace drafts or successful UI
  responses.
- No profile rename/delete management flow.
- No restoration of historical error banners, stale toasts, or failed request
  payloads across profile switches.

## Acceptance Criteria

- The shell presents profile, host, health, and workspace context with stronger
  navigation cues and accessible focus/alert states.
- Profile selection uses a dropdown for known projects and a separate confirmed
  input for new project IDs; typing alone never hot-switches the active project.
- Switching projects triggers visible summary loading, keeps current-page data in
  sync, and shows a clear uninitialized guide when the project has no inventory.
- Overview reads as an operator runway with KPI summary, action cards, and clear
  warning/next-step surfaces.
- IP Extract restores per-project filter/result state and stays usable on mobile.
- Sessions restores per-project open/batch form state and keeps live-listener
  polling scoped to the active project.
- Storybook and automated checks cover refreshed component/page states, and the
  smoke flow remains green.

## Verification

- `bun run check`
- `bun run typecheck`
- `bun run test`
- `bun run verify:stories`
- `bun run build`
- `bun run build-storybook`
- `bun run test:e2e`

## Outcome

- The control-room shell, overview runway, IP extract workspace, and sessions
  workspace are implemented on the current PR branch with project-level profile
  discovery and summary APIs.
- Shared field controls now use an explicit size system so large trigger,
  content, and item surfaces stay visually consistent across the real app and
  Storybook.
- Route-level UI summaries and successful workspace results stay scoped to the
  profile that produced them, preventing stale cross-profile state from leaking
  into the operator panels while still restoring that state when the operator
  returns to the same project.
