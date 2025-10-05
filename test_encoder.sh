#!/bin/bash
set -e

echo "Testing encoder with recording from recorder..."

# Make sure we have a test recording
if [ ! -f /tmp/test_recorder.mp4 ]; then
  echo "No test recording found. Run ./test_recorder.sh first"
  exit 1
fi

# Test 1: GIF encoding
echo "Test 1: Encoding GIF..."
echo '{"cmd":"gif","in":"/tmp/test_recorder.mp4","trim":{"start_ms":0,"end_ms":2700},"fps":15,"scale_px":480,"loop":"normal","captions":[],"out":"/tmp/test.gif"}' | cargo run --bin encoder --quiet 2>&1

if [ -f /tmp/test.gif ]; then
  SIZE=$(stat -c%s /tmp/test.gif 2>/dev/null || stat -f%z /tmp/test.gif)
  echo "✓ GIF created: /tmp/test.gif (${SIZE} bytes)"
else
  echo "✗ GIF encoding failed"
  exit 1
fi

# Test 2: WebP encoding
echo "Test 2: Encoding WebP..."
echo '{"cmd":"webp","in":"/tmp/test_recorder.mp4","trim":{"start_ms":0,"end_ms":2700},"fps":15,"scale_px":480,"quality":0.85,"lossless":false,"captions":[],"out":"/tmp/test.webp"}' | cargo run --bin encoder --quiet 2>&1

if [ -f /tmp/test.webp ]; then
  SIZE=$(stat -c%s /tmp/test.webp 2>/dev/null || stat -f%z /tmp/test.webp)
  echo "✓ WebP created: /tmp/test.webp (${SIZE} bytes)"
else
  echo "✗ WebP encoding failed"
  exit 1
fi

# Test 3: MP4 re-encoding
echo "Test 3: Re-encoding MP4..."
echo '{"cmd":"mp4","in":"/tmp/test_recorder.mp4","trim":{"start_ms":0,"end_ms":2700},"fps":30,"scale_px":640,"quality":0.8,"captions":[],"out":"/tmp/test_reencoded.mp4"}' | cargo run --bin encoder --quiet 2>&1

if [ -f /tmp/test_reencoded.mp4 ]; then
  SIZE=$(stat -c%s /tmp/test_reencoded.mp4 2>/dev/null || stat -f%z /tmp/test_reencoded.mp4)
  echo "✓ MP4 created: /tmp/test_reencoded.mp4 (${SIZE} bytes)"
else
  echo "✗ MP4 encoding failed"
  exit 1
fi

echo ""
echo "✓ All encoder tests passed"
echo "Output files:"
echo "  - /tmp/test.gif"
echo "  - /tmp/test.webp" 
echo "  - /tmp/test_reencoded.mp4"