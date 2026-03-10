# Build Scan Server

Axum-based HTTP server that ingests Gradle build scans, stores them in SQLite, exposes a GraphQL API, and serves the Angular frontend as a SPA.

## Targets

### Run (development)

```bash
ibazel run //build-scan/server/src:main   # Hot-reload
bazel run //build-scan/server/src:main    # Single run
```

### Docker Image

```bash
aspect build //build-scan/server/src:image        # Build OCI image
bazel run //build-scan/server/src:load_image      # Load into local Docker
bazel run //build-scan/server/src:push_image      # Push to ghcr.io/lowkeylab/build-scan-server
```

Run the container:

```bash
docker run -p 8080:8080 -v ./data:/data local/build-scan-server:latest
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Server listen port |
| `DATABASE_URL` | `sqlite:///data/build-scans.db?mode=rwc` | SQLite connection string |
| `SPA_DIR` | `/app/browser` | Path to Angular frontend dist |
| `BASE_URL` | `http://localhost:{PORT}` | Public URL for scan upload tokens |

All are baked into the Docker image with sensible defaults. Override with `-e` at runtime.

## Crate Layout

| Crate | Path | Purpose |
|-------|------|---------|
| `main` | `src/` | Axum router, server entrypoint |
| `config` | `config/src/` | Env-based configuration |
| `db` | `db/src/` | SQLite pool + queries (sqlx) |
| `domain` | `domain/src/` | Domain models |
| `graphql` | `graphql/src/` | Juniper schema + resolvers |
| `ingest` | `ingest/src/` | Build scan upload protocol |
| `service` | `service/src/` | Business logic layer |

## Endpoints

- `POST /scans/publish/{tool_type}/{version}/token` — Request upload token
- `POST /scans/publish/{tool_type}/{version}/upload` — Upload build scan
- `GET /usage/users/check` — Usage check
- `GET|POST /graphql` — GraphQL API
- `GET /graphiql` — GraphiQL IDE
- `GET /web/*` — Angular SPA (when `SPA_DIR` is set)
