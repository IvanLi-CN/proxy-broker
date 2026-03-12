# proxy-broker web

The `web/` workspace hosts the operator console for `proxy-broker`.

## Stack

- Bun 1.x
- Vite 7 + React 19 + TypeScript
- Tailwind CSS 4 + shadcn/ui
- TanStack Query + React Hook Form + Zod
- Storybook 10 with autodocs and interaction tests
- Vitest + Testing Library + Playwright

## Local development

Install dependencies with Bun:

```bash
cd web
bun install
```

Start the app against the local Rust API:

```bash
bun run dev
```

The Vite app binds `127.0.0.1:38181` and proxies `/api` plus `/healthz` to `127.0.0.1:8080`.

Start Storybook on `127.0.0.1:38182`:

```bash
bun run storybook
```

## Required checks

```bash
bun run check
bun run test
bun run verify:stories
bun run build-storybook
bun run test-storybook
bun run build
bun run test:e2e
```

## Storybook contract

Every UI component or page committed under these paths must ship with colocated
`*.stories.tsx` files and Storybook autodocs metadata:

- `src/components/ui/**`
- `src/components/**`
- `src/features/**/components/**`
- `src/pages/**`

The `scripts/verify-stories.ts` gate fails when a covered component has no
story file or when the story disables autodocs metadata.
