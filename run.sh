#!/usr/bin/env bash
# launch.sh — clean, build, deploy to Quest, and open mirror window

set -e

QUEST_APP_DIR="$HOME/Desktop/rampstack/quest_app"
MIRROR_DIR="$HOME/Desktop/rampstack/mirror"
SPACE_SOUP_DIR="$HOME/Desktop/rampstack/space_soup"
HOST_TARGET=$(rustc -vV | awk '/host:/ {print $2}')

# ── 1. Clean all projects ─────────────────────────────────────────────────────
echo "==> Cleaning space_soup..."
cd "$SPACE_SOUP_DIR"
cargo clean

echo "==> Cleaning mirror..."
cd "$MIRROR_DIR"
cargo clean

echo "==> Cleaning quest_app..."
cd "$QUEST_APP_DIR"
cargo clean
cd android && ./gradlew clean && cd ..

# ── 2. Pre-build mirror so cargo run starts instantly later ───────────────────
echo "==> Pre-building mirror..."
cd "$MIRROR_DIR"
cargo build --target "$HOST_TARGET"

# ── 3. Build and deploy to Quest ─────────────────────────────────────────────
echo "==> Building quest_app for Android..."
cd "$QUEST_APP_DIR"
cargo build --target aarch64-linux-android --release
mkdir -p android/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libquest_app.so android/jniLibs/arm64-v8a/
cd android
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
cd "$QUEST_APP_DIR"

# ── 4. Set up ADB reverse ─────────────────────────────────────────────────────
echo "==> Reversing TCP pose port 7777..."
adb reverse tcp:7777 tcp:7777

# ── 5. Open Terminal windows ──────────────────────────────────────────────────

# Terminal 1 — logcat
osascript <<EOF
tell application "Terminal"
    do script "cd $QUEST_APP_DIR && adb logcat -s quest_app"
    set bounds of front window to {0, 0, 900, 500}
end tell
EOF

# Terminal 2 — ADB reverse keepalive
osascript <<EOF
tell application "Terminal"
    do script "while true; do adb reverse tcp:7777 tcp:7777 2>/dev/null; sleep 5; done"
    set bounds of front window to {0, 520, 900, 800}
end tell
EOF

# Terminal 3 — Mirror window (already built, starts instantly)
osascript <<EOF
tell application "Terminal"
    do script "cd $MIRROR_DIR && cargo run --target $HOST_TARGET"
    set bounds of front window to {920, 0, 1800, 500}
end tell
EOF

# ── 6. Wait for mirror listener to be ready ───────────────────────────────────
echo "==> Waiting for mirror to start listening (3s)..."
sleep 3

# ── 7. Launch Quest app ───────────────────────────────────────────────────────
echo "==> Launching quest_app on headset..."
adb shell am start -n com.example.questapp/android.app.NativeActivity

echo "==> All terminals launched."
echo "    Put on your headset and move around — mirror window tracks your view."