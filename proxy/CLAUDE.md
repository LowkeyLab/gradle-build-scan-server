# Proxy

HTTP intercepting proxy that sits between Gradle clients and an upstream build scan server. Captures all request/response traffic to a local SQLite database and exposes it via a GraphQL API.

## Targets

### Run (development)

```bash
ibazel run //proxy/src:main   # Hot-reload
bazel run //proxy/src:main    # Single run
```

### Environment Variables

| Variable       | Default           | Description                    |
| -------------- | ----------------- | ------------------------------ |
| `PORT`         | `8080`            | Server listen port             |
| `UPSTREAM_URL` | _(required)_      | Upstream build scan server URL |
| `DATABASE_URL` | `sqlite:proxy.db` | SQLite connection string       |

## Crate Layout

| Crate     | Path                 | Purpose                                            |
| --------- | -------------------- | -------------------------------------------------- |
| `main`    | `src/`               | Axum router, proxy handler, server entrypoint      |
| `config`  | `config/src/`        | Env-based configuration                            |
| `db`      | `db/src/`            | SQLite pool + CRUD queries (sqlx)                  |
| `domain`  | `domain/src/`        | Domain models (Payload, RequestData, ResponseData) |
| `format`  | `format/src/`        | Wire format types for JSON serialization           |
| `service` | `service/src/`       | Business logic — transforms format types to domain |
| `graphql` | `graphql/src/`       | Juniper schema + resolvers                         |
| `relay`   | `graphql/relay/src/` | Relay Global Object ID + cursor pagination         |

## Data Flow

```
Client Request
  → proxy_handler captures method/uri/headers/body
  → forwards to UPSTREAM_URL via reqwest
  → captures response status/headers/body
  → ProxyService::store_payload() → SQLite INSERT
  → returns upstream response to client
```

## GraphQL API

- `node(id: ID!): Node` — Relay global ID resolution
- `payload(id: ID!): Payload` — Direct UUID or Relay ID lookup
- `payloads(first: Int, after: String): PayloadConnection!` — Cursor pagination

## Database Schema

Single `payloads` table with columns: id (UUID), request_id, timestamp, method, uri, request_headers (JSON), request_body, response_status, response_headers (JSON), response_body, response_error, created_at.
