#!/bin/bash
set -e

# Clean up any previous test files
rm -f /tmp/test_recorder.mp4

# Build first
echo "Building recorder..."
cargo build --bin recorder-linux --quiet

# Run recorder with automatic start/stop
echo "Starting 3-second test recording..."
(
  echo '{"cmd":"start","region":{"x":100,"y":100,"width":640,"height":480},"fps":30,"cursor":false,"out":"/tmp/test_recorder.mp4"}'
  sleep 3
  echo '{"cmd":"stop"}'
) | cargo run --bin recorder-linux --quiet 2>&1

# Verify the file exists and is valid
if [ -f /tmp/test_recorder.mp4 ]; then
  SIZE=$(stat -f%z /tmp/test_recorder.mp4 2>/dev/null || stat -c%s /tmp/test_recorder.mp4)
  echo "✓ Recording created: /tmp/test_recorder.mp4 (${SIZE} bytes)"
  
  # Check if ffprobe is available
  if command -v ffprobe &> /dev/null; then
    echo "✓ Validating with ffprobe..."
    ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 /tmp/test_recorder.mp4
  fi
  
  echo "✓ Test passed - you can play: mpv /tmp/test_recorder.mp4"
else
  echo "✗ Test failed - no output file created"
  exit 1
fi