# sw-head-tracking

Head tracking for [Stormworks: Build and Rescue](https://store.steampowered.com/app/573090/).

> **Status:** Functional, under active development.

## Requirements

- Stormworks (Steam, Windows or Linux via Proton)
- A FreeTrack compatible head tracker.
- To build from source: a recent Rust toolchain, `cmake` and a C compiler (see below)

## Installation

### Prebuilt

Download the latest zip from the [Releases page](https://github.com/rar61/sw-head-tracking/releases/latest) and extract it.

### Building from source

```bash
cargo build --release
```

| Setup                                  | What you need                                                                                       |
|----------------------------------------|-----------------------------------------------------------------------------------------------------|
| Default (builds `minhook` from source) | `cmake` and a C compiler that `cmake` can find                                                      |
| Your own `minhook` library             | Disable the `build-from-source` feature in `Cargo.toml`; neither `cmake` nor a C compiler is needed |

#### Cross-compiling from Linux

Install `cmake` and MinGW-w64, then:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

### Copying files into place

Copy `sw-loader.exe` and `sw_head_tracking.dll` (from the extracted zip, or from your build output folder) into the Stormworks install folder (the one containing the game executable).

## Usage

### Windows

1. Start your head tracker.
2. Run `sw-loader.exe`. It launches and patches the game automatically.

**Tip:** Add `sw-loader.exe` to Steam as a *Non-Steam Game* to start it from your library.

### Linux (Proton)

The injector must run inside the same Proton prefix as the game.

1. In Steam, choose **Add a Non-Steam Game** and select `sw-loader.exe`.
2. Open its **Properties > Compatibility** and force the same Proton version Stormworks uses.
3. Set the **Launch Options** to point at the Stormworks prefix:

```bash
   STEAM_COMPAT_DATA_PATH=~/.steam/root/steamapps/compatdata/573090/ %command%
```

`573090` is Stormworks' Steam App ID. Your path may differ depending on your Steam install.
