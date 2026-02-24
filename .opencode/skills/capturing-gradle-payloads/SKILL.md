---
name: capturing-gradle-payloads
description: Use when you need to capture the JSON payloads sent by a Gradle build scan to the server for reverse engineering or debugging purposes
---

# Capturing Gradle Payloads

## Overview
This skill provides a standard way to intercept and record JSON payloads emitted by a Gradle build when `--scan` is run. It uses the project's proxy-based `echo-server` to forward traffic to `scans.gradle.com` while silently saving copies of requests and responses.

## When to Use
- You need to see the exact structure of a Gradle build scan payload.
- You are debugging discrepancies between local builds and Gradle Enterprise/Develocity.
- You are adding new reverse-engineering models for build scan metrics.
- A user asks to "run the echo-server" to capture output.

## Core Principle
Instead of manually wiring up port bindings, environment variables, and manual file copies, always use the automated capture script bundled with this skill. It sets up the environment, builds the server, kicks off Gradle, cleans up the processes, and extracts the results into a single directory.

## Implementation

The process is fully automated by a bash script bundled with this skill.

```bash
# Execute the capture script from the repository root
./.opencode/skills/capturing-gradle-payloads/capture.sh
```

**What it does:**
1. Cleans the temporary payload directory (`/tmp/gradle-payloads`).
2. Builds `//echo-server/src:main` using Bazel.
3. Spawns the `echo-server` in the background (proxying to `https://scans.gradle.com` on port `8080`).
4. Executes `./gradlew build --scan --no-build-cache` in the `gradle/` directory with `DEVELOCITY_SERVER_URL` pointing to the local proxy.
5. Terminates the `echo-server` background process.
6. Aggregates all captured JSON payloads and execution logs into `./captured-output/`.

## Inspecting the Published Build Scan

After capturing payloads, the Gradle output log contains a URL to the published build scan (e.g., `https://scans.gradle.com/s/...`). Use browser automation to open and inspect the published scan page — extract the URL, navigate to it, take a snapshot, and capture page content or screenshots.

### Monitoring Network Traffic via Browser

If you need to observe the network requests the build scan page itself makes (as opposed to the Gradle client payloads captured by the echo-server), use browser profiling to capture a network trace while the page loads.

### Routing Browser Traffic Through the Echo-Server Proxy

You can route browser traffic through the local echo-server proxy (on port `8080`) to capture browser-originated requests. Start the echo-server first (the `capture.sh` script does this automatically), then open a browser session with the proxy set to `http://localhost:8080`.

## Common Mistakes

| Mistake | Correction |
|---------|------------|
| Leaving the server running | The `capture.sh` script automatically tracks and kills the background PID. Don't run it manually if possible to avoid zombie processes. |
| Missing `UPSTREAM_URL` | The server requires this env var. The script injects it automatically. |
| Missing `DEVELOCITY_SERVER_URL` | The Gradle build only points to the local proxy if this env var is set. The script injects it automatically. |
| Forgetting `--no-build-cache` | Gradle might skip the scan entirely if everything is cached. The script ensures a full run. |
| Not closing browser sessions | Always close your browser session when done to avoid leaked processes. |

## Quick Reference
- **Script Location**: `./.opencode/skills/capturing-gradle-payloads/capture.sh`
- **Output Directory**: `./captured-output/`
- **Payload Location**: `./captured-output/payloads/*.json`
- **Server Logs**: `./captured-output/echo-server-output.log`
- **Gradle Logs**: `./captured-output/gradle-build-output.log`
