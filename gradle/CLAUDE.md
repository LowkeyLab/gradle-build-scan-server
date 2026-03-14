# Gradle Test Project

Standalone Gradle/Kotlin multi-module project used for **dogfooding** — it publishes build scans to the build scan server, generating real traffic for testing and development.

This is NOT part of the build scan server itself. It exists to produce Gradle build events that the server can ingest.

## Targets

### Build & Test

```bash
cd gradle && ./gradlew build   # Build all modules and run tests
cd gradle && ./gradlew :app:run   # Run the CLI app
```

### Publishing Build Scans

Set `DEVELOCITY_SERVER_URL` to point at your local build scan server:

```bash
DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew build
```

## Module Structure

| Module | Type | Purpose |
|--------|------|---------|
| `app/` | Application | CLI entry point — splits, joins, and capitalizes strings |
| `utilities/` | Library | String utility facades (split/join via LinkedList) |
| `list/` | Library | Custom doubly-linked list implementation |
| `build-logic/` | Convention plugins | Shared build configuration (Kotlin JVM, Java 21 toolchain, JUnit 5) |

## Build Configuration

- **Gradle** with Kotlin DSL and convention plugins
- **Develocity plugin** v4.3.2 configured in `settings.gradle.kts`
- **Configuration cache**, parallel execution, and local caching enabled via `gradle.properties`
- **Java 21** toolchain
