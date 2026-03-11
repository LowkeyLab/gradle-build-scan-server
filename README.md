# Gradle Build Scan Server

A self-hosted server for receiving and viewing Gradle build scans.

## Docker

### Pull and run

```bash
docker pull ghcr.io/lowkeylab/build-scan-server:latest
docker run -p 8080:8080 -v build-scan-data:/data ghcr.io/lowkeylab/build-scan-server:latest
```

The server will be available at `http://localhost:8080`.

### Configuration

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:///data/build_scans.db` | SQLite database path |
| `PORT` | `8080` | HTTP listen port |

Mount a volume at `/data` for persistent storage:

```bash
docker run -p 3000:3000 \
  -e PORT=3000 \
  -v build-scan-data:/data \
  ghcr.io/lowkeylab/build-scan-server:latest
```

### Build locally

```bash
bazel run //build-scan/server/src:load_image   # loads as local/build-scan-server:latest
docker run -p 8080:8080 -v build-scan-data:/data local/build-scan-server:latest
```
