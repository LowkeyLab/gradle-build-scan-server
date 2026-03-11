# Gradle Build Scan Server

A self-hosted server for receiving and viewing Gradle build scans.

## Getting Started

### 1. Start the server

```bash
docker run -p 8080:8080 -v build-scan-data:/data ghcr.io/lowkeylab/build-scan-server:latest
```

### 2. Configure your Gradle project

Add the [Develocity plugin](https://docs.gradle.com/develocity/gradle-plugin/) to your `settings.gradle.kts`:

```kotlin
plugins {
    id("com.gradle.develocity") version "4.3.2"
}

develocity {
    server = "http://localhost:8080"
    buildScan {
        publishing.onlyIf { true }
        uploadInBackground = false
    }
}
```

### 3. Run a build

```bash
./gradlew build
```

The build scan will be uploaded to your server and viewable at `http://localhost:8080`.

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
