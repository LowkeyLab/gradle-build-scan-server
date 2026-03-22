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
        └── scan-detail.component.ts     # /scans/:id — scan metadata, tasks, tests
```

## Routes

| Route        | Component           | Description                              |
| ------------ | ------------------- | ---------------------------------------- |
| `/scans`     | ScanListComponent   | Paginated list of build scans (20/page)  |
| `/scans/:id` | ScanDetailComponent | Scan details with tasks and tests tables |

## GraphQL Queries

- `GetBuildScans(first, after)` — Relay cursor pagination for scan list
- `GetBuildScan(id, firstTasks, afterTasks, firstTests, afterTests)` — Single scan with nested task/test connections

## Backend Integration

The app is served by the Rust server at `/web/*` when `SPA_DIR` is configured. The GraphQL endpoint is resolved relative to `document.baseURI` as `/graphql`.
