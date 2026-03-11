# Proxy Service with GraphQL API

## Summary

Convert the proxy from a JSON-file-dumping interceptor into a proper service with SQLite persistence and a GraphQL API for querying captured payloads. The proxy remains a separate binary from the build-scan server.

## Context

The proxy (`proxy/`) currently forwards HTTP requests to an upstream server and saves each request/response pair as a JSON file on disk. This is useful for debugging but the data is not queryable or persistent in a structured way.

The build-scan server (`build-scan/server/`) already has a mature pattern: Juniper GraphQL with Relay pagination, SQLite via sqlx, and a layered architecture (domain → db → service → graphql). The proxy should follow the same patterns.

## Design

### Activation

Proxy mode activates when `UPSTREAM_URL` is set. Without it, the server does not start (it has no purpose without an upstream to forward to).

### Architecture

Two independent services, each with their own database and GraphQL endpoint:

```
proxy/src:main (port 8080)          build-scan/server/src:main (port 3000)
├── Proxy handler (forward+capture) ├── Build scan receiver
├── GraphQL API (/graphql)          ├── GraphQL API (/graphql)
├── GraphiQL IDE (/graphiql)        ├── GraphiQL IDE (/graphiql)
└── SQLite (payloads)               └── SQLite (build_scans, tasks, tests)
```

### New Modules

Follow the build-scan server's layered structure, using the same naming conventions:

```
proxy/
├── config/src/lib.rs       # Update: add DATABASE_URL config
├── format/src/lib.rs       # Existing: payload structs (keep as-is)
├── domain/src/lib.rs       # New: domain models for DB representation
├── db/src/lib.rs           # New: SQLite persistence via sqlx
├── service/src/lib.rs      # New: business logic layer
├── graphql/src/lib.rs      # New: Juniper schema + resolvers
├── graphql/relay/src/lib.rs # New: Relay utilities (duplicated from build-scan server)
└── src/main.rs             # Update: add DB init, GraphQL routes, remove JSON dumping
```

### Database Schema

Single `payloads` table. Migrations live in `proxy/db/migrations/` and are applied via `sqlx::migrate!()`.

```sql
CREATE TABLE payloads (
    id TEXT PRIMARY KEY,           -- UUID
    request_id TEXT NOT NULL,      -- original UUID from proxy
    timestamp TEXT NOT NULL,       -- ISO 8601
    method TEXT NOT NULL,
    uri TEXT NOT NULL,
    request_headers TEXT NOT NULL,  -- JSON array of {"name":"...","value":"..."} objects
    request_body TEXT,             -- JSON string, NULL when body is empty
    response_status INTEGER,       -- u16 widened to INTEGER (safe, no data loss)
    response_headers TEXT,         -- JSON array of {"name":"...","value":"..."} objects
    response_body TEXT,            -- JSON string, NULL when body is empty
    response_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Type mappings:**
- `format::RequestData.body` (`serde_json::Value`) → stored as JSON string; `Value::Null` maps to SQL NULL
- `format::ResponseData.status` (`Option<u16>`) → stored as INTEGER (i32 widening, safe)
- Headers (`Vec<(String, String)>`) → serialized as `[{"name":"...","value":"..."}]` to match the GraphQL `Header` type directly

### GraphQL Schema

```graphql
interface Node {
  id: ID!
}

type Query {
  node(id: ID!): Node
  payloads(first: Int, after: String): PayloadConnection!
  payload(id: ID!): Payload
}

type Payload implements Node {
  id: ID!
  requestId: String!
  timestamp: String!
  request: RequestData!
  response: ResponseData!
}

type RequestData {
  method: String!
  uri: String!
  headers: [Header!]!
  body: String
}

type ResponseData {
  status: Int
  headers: [Header!]
  body: String
  error: String
}

type Header {
  name: String!
  value: String!
}

type PayloadConnection {
  edges: [PayloadEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type PayloadEdge {
  node: Payload!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  endCursor: String
}
```

Relay pagination with cursor-based navigation, matching the build-scan server's implementation. Cursors encode the payload `id` in base64.

### Request Flow

1. Gradle plugin sends request to proxy
2. Proxy middleware captures request data
3. Proxy forwards request to upstream (`UPSTREAM_URL`)
4. Upstream responds
5. Proxy captures response data
6. Proxy stores complete payload to SQLite
7. Proxy returns upstream response to client

### Config Changes

Add to `proxy/config/src/lib.rs`:

- `database_url`: String (env: `DATABASE_URL`, default: `sqlite:proxy.db`)

Remove:

- `payload_dir`: no longer needed (JSON file dumping removed)

### What Gets Removed

- JSON file dumping from `proxy/src/main.rs`
- `payload_dir` config option
- Filesystem-based payload storage (SQLite replaces it)

### What Gets Reused

- `proxy/format` structs — used as intermediate representation between capture and domain models
- Juniper + Axum patterns from build-scan server

### Decisions

**Relay utilities:** Duplicate the ~100 lines from `build-scan/server/graphql/relay/` into `proxy/graphql/relay/`. Simple, no cross-dependency. Extract to a shared crate later if a third consumer appears.

### Dependencies

New crate dependencies for the proxy:

- `sqlx` (SQLite, runtime-tokio)
- `juniper` + `juniper_axum`
- Existing: `axum`, `tokio`, `reqwest`, `serde`, `serde_json`, `tracing`
