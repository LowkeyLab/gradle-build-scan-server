# Angular Frontend

Angular 21 SPA for browsing Gradle build scans. Communicates exclusively via GraphQL with the Rust backend.

## Targets

### Build

```bash
aspect build //angular/projects/build-scan-web   # Build production app
```

### Test

```bash
aspect test //angular/projects/build-scan-web:test   # Run Vitest unit tests
```

## Tech Stack

| Layer     | Technology                                                           |
| --------- | -------------------------------------------------------------------- |
| Framework | Angular 21 (standalone components, zoneless change detection)        |
| State     | Apollo Client (GraphQL-centric, no NgRx)                             |
| Styling   | Tailwind CSS + DaisyUI                                               |
| Testing   | Vitest                                                               |
| Build     | Bazel via custom `ng_application` / `ng_test` macros in `bzl/ng.bzl` |

## Project Structure

```
angular/
├── bzl/ng.bzl                           # Bazel macros (ng_application, ng_test, process_styles)
└── projects/build-scan-web/src/app/
    ├── app.ts                           # Root component (RouterOutlet)
    ├── app.routes.ts                    # Route definitions
    ├── app.config.ts                    # Providers (zoneless, router, http, graphql)
    ├── graphql.provider.ts              # Apollo Client setup (URI from document.baseURI)
    └── scans/
        ├── scan-list.component.ts       # /scans — paginated scan list
        ├── scan-detail.component.ts     # /scans/:id — orchestrator (GraphQL query + child layout)
        ├── build-metadata/              # Build outcome, timestamps, tool/OS/JVM info
        ├── task-timeline/               # Gantt-style task execution timeline
        ├── cache-breakdown/             # Cache hit/miss waffle chart (Observable Plot)
        ├── tasks-table/                 # Task list with caching status badges
        └── tests-table/                 # Test results with outcome badges
```

## Routes

| Route        | Component           | Description                              |
| ------------ | ------------------- | ---------------------------------------- |
| `/scans`     | ScanListComponent   | Paginated list of build scans (20/page)  |
| `/scans/:id` | ScanDetailComponent | Scan details with tasks and tests tables |

## GraphQL Queries

- `GetBuildScans(first, after)` — Relay cursor pagination for scan list
- `GetBuildScan(id, firstTasks, afterTasks, firstTests, afterTests)` — Single scan with nested task/test connections

## Visual Verification (E2E)

After changing frontend components, verify the UI renders correctly using `agent-browser`:

```bash
# 1. Start the server (from repo root)
bazel run //build-scan/server/src:main

# 2. Publish a test build scan
cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean build

# 3. Open the scan URL printed by Gradle and verify with agent-browser
agent-browser open http://localhost:3000/web/scans/<scan-id>
agent-browser wait --load networkidle
agent-browser wait 2000
agent-browser eval --stdin <<'EOF'
JSON.stringify({
  metadata: document.querySelector("app-build-metadata") !== null,
  timeline: document.querySelector("app-task-timeline") !== null,
  cacheBreakdown: document.querySelector("app-cache-breakdown") !== null,
  tasksTable: document.querySelector("app-tasks-table") !== null,
  testsTable: document.querySelector("app-tests-table") !== null,
  testDurationColumn: Array.from(document.querySelectorAll("th")).some(h => h.textContent.trim() === "Duration"),
  testSummaryBadges: document.querySelectorAll(".badge-lg").length
})
EOF
# Expected: all true, testSummaryBadges >= 3

# 4. Take a screenshot (use dark theme for visibility in headless)
agent-browser eval 'document.documentElement.setAttribute("data-theme", "dark")'
agent-browser screenshot --full /tmp/scan-detail.png

# 5. Clean up
agent-browser close
```


## Adding npm Dependencies

Bazel sandboxing requires ALL transitive deps to be explicit. When adding a package with deep dependency trees (e.g., `@observablehq/plot` → `d3` → 30+ `d3-*` sub-packages):

1. Add to `pnpm-workspace.yaml` catalog AND `angular/package.json`
2. Add `//angular:node_modules/<pkg>` to `BUILD.bazel` deps
3. **Every transitive dep** that the bundler resolves also needs steps 1-2
4. Use `npm view <pkg> dependencies --json` to discover transitive deps upfront
5. Group transitive deps in a `_DEPS` list variable in BUILD.bazel for maintainability

## Angular 21 API Notes

- `afterRender` and `AfterRenderPhase` no longer exist — use `afterNextRender`, `afterEveryRender`, or `afterRenderEffect`
- `afterRenderEffect` is signal-reactive (re-runs when read signals change) — ideal for imperative DOM libraries like Observable Plot
- For libraries returning detached DOM elements (e.g., `Plot.plot()`), use `viewChild` + `afterRenderEffect` + `el.replaceChildren()`

## Backend Integration

The app is served by the Rust server at `/web/*` when `SPA_DIR` is configured. The GraphQL endpoint is resolved relative to `document.baseURI` as `/graphql`.
