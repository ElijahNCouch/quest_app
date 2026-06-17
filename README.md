# quest_app

A Meta Quest VR app built with [RSXR](../rsxr).

## Prerequisites

```sh
# 1. Rust Android target
rustup target add aarch64-linux-android

# 2. Android NDK (via Android Studio SDK Manager, or direct download)
#    Set ANDROID_NDK_HOME in your shell:
export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/26.x.x

# 3. Update .cargo/config.toml linker version to match your NDK
#    e.g. aarch64-linux-android35-clang → match your API level
```

## Build & Deploy

```sh
# One command — builds, packages, signs, installs, and tails logcat
./build.sh
```

Or step by step:

```sh
# Build the .so
cargo build --target aarch64-linux-android --release --lib

# Check it compiled
file target/aarch64-linux-android/release/libquest_app.so
# → ELF 64-bit LSB shared object, ARM aarch64

# Sideload manually
adb install build/quest_app.apk
adb logcat -s quest_app:V OpenXR:V
```

## Enable Developer Mode on Quest

1. Open the Meta app on your phone
2. Go to **Menu → Devices → [your headset] → Developer Mode → On**
3. Plug in via USB, accept the prompt inside the headset
4. `adb devices` should show your headset

## Project Structure

```
quest_app/
├── src/
│   ├── lib.rs          ← shared app logic + android_main entry point
│   └── main.rs         ← desktop entry point (for local testing)
├── android/
│   └── app/src/main/
│       └── AndroidManifest.xml
├── .cargo/
│   └── config.toml     ← Android linker config
├── build.sh            ← full build + deploy script
└── Cargo.toml
```

## Next Steps

The `run_inner()` function in `src/lib.rs` has commented scaffolding for:

- Creating a Vulkan device
- Initialising an OpenXR session
- The full frame loop with hand input
- Rendering to the swapchain

Uncomment and fill in the Vulkan init to get frames rendering.
