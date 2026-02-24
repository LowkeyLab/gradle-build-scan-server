#!/bin/bash
set -e

# Default variables
UPSTREAM_URL=${UPSTREAM_URL:-"https://scans.gradle.com"}
PORT=${PORT:-8080}
PAYLOAD_DIR=${PAYLOAD_DIR:-"/tmp/gradle-payloads"}
OUTPUT_DIR=${OUTPUT_DIR:-"captured-output"}

# Get workspace root
WORKSPACE_DIR=$(git rev-parse --show-toplevel)

echo "Cleaning up old payloads in $PAYLOAD_DIR..."
rm -rf "$PAYLOAD_DIR"
mkdir -p "$PAYLOAD_DIR"

echo "Building echo-server..."
cd "$WORKSPACE_DIR"
bazel build //proxy/src:main

echo "Starting echo-server proxy to $UPSTREAM_URL on port $PORT..."
UPSTREAM_URL="$UPSTREAM_URL" PORT="$PORT" PAYLOAD_DIR="$PAYLOAD_DIR" "$WORKSPACE_DIR/bazel-bin/proxy/src/main" >echo-server-output.log 2>&1 &
SERVER_PID=$!

echo "Waiting for server to start..."
sleep 3

echo "Running gradle build in $WORKSPACE_DIR/gradle..."
cd "$WORKSPACE_DIR/gradle"
# Using Gradle configuration cache, skip tasks if needed to trigger scan
DEVELOCITY_SERVER_URL="http://localhost:$PORT" ./gradlew build --scan --no-build-cache >"$WORKSPACE_DIR/gradle-build-output.log" 2>&1 || true

echo "Killing echo-server (PID: $SERVER_PID)..."
kill $SERVER_PID || true

echo "Saving payloads to $OUTPUT_DIR..."
cd "$WORKSPACE_DIR"
mkdir -p "$OUTPUT_DIR/payloads"
cp -r "$PAYLOAD_DIR"/* "$OUTPUT_DIR/payloads/" 2>/dev/null || echo "No payloads found."
mv echo-server-output.log "$OUTPUT_DIR/" 2>/dev/null || true
mv gradle-build-output.log "$OUTPUT_DIR/" 2>/dev/null || true

echo "Capture complete. Outputs stored in $OUTPUT_DIR/"
