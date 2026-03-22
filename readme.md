# tiled_map_web_viewer

[![license](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE) <img src="https://img.shields.io/github/repo-size/Bli-AIk/tiled_map_web_viewer.svg"/> <img src="https://img.shields.io/github/last-commit/Bli-AIk/tiled_map_web_viewer.svg"/> <br>
<img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" /> <img src="https://img.shields.io/badge/Bevy-232326?style=for-the-badge&logo=bevy&logoColor=white" />

> Current Status: 🚧 Early Development (Initial version in progress)

**tiled_map_web_viewer** — A web-based Tiled map viewer designed for open-source map library projects to preview their map collections via GitHub Pages.

| English  | Simplified Chinese                     |
|----------|----------------------------------------|
| English  | [简体中文](./readme_zh-hans.md)         |

## Introduction

`tiled_map_web_viewer` is a lightweight Tiled map viewer built with Bevy and WebAssembly.  
It is designed for open-source game map library maintainers who want to provide an interactive web preview of their `.tmx` and `.world` map collections directly on GitHub Pages.

With `tiled_map_web_viewer`, you only need to deploy a WASM build alongside your map assets to give contributors and users a visual browser for your map library — no desktop installation required.

The repository also bundles sample assets derived from [The Mana World client data](https://github.com/themanaworld/tmwa-client-data). See [assets/NOTICE](assets/NOTICE) for attribution and license details.

## Why Tiled?

[Tiled](https://www.mapeditor.org/) is a free, open-source 2D map editor that has become the de facto standard in the indie and open-source game development community. Its `.tmx` map format is supported by virtually every major 2D game engine and framework — including Godot, Unity, Phaser, LÖVE, pygame, libGDX, and many more.

By choosing Tiled as the base format, `tiled_map_web_viewer` can **adapt to the diverse creation toolchains used across different game communities**. Whether your contributors use Godot, RPG Maker (with TMX export), or Tiled itself, as long as maps can be exported to `.tmx`, they can be previewed here. This makes it an ideal tool for open-source map library projects that serve multiple engines and workflows.

## Features

* **Map List Panel** — Browse all available maps in the library and click to preview
* **Tiled Map Rendering** — Powered by `bevy_ecs_tiled`, supports orthogonal, isometric, and hexagonal maps
* **Editor-Style UI** — Dock layout with panels, powered by `bevy_workbench`
* **Layer Visibility Control** — Toggle individual map layers on/off
* **Camera Controls** — Zoom and pan to explore maps
* **Desktop & WASM** — Run natively or deploy to GitHub Pages
* **World Support** — Load `.world` files and preview stitched multi-map layouts

## How to Use

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/Bli-AIk/tiled_map_web_viewer.git
   cd tiled_map_web_viewer
   ```

3. **Run (desktop)**:
   ```bash
   cargo run
   ```

4. **Run (WASM)**:
   ```bash
   cargo install trunk
   rustup target add wasm32-unknown-unknown
   trunk serve
   ```

## How to Build

### Prerequisites

* Rust 1.85 or later
* Bevy 0.18 compatible system dependencies:
  ```bash
  # Linux (Ubuntu/Debian)
  sudo apt-get install -y g++ pkg-config libx11-dev libasound2-dev libudev-dev \
      libwayland-dev libxkbcommon-dev
  ```

### Build Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/Bli-AIk/tiled_map_web_viewer.git
   cd tiled_map_web_viewer
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

3. **Build for WASM**:
   ```bash
   trunk build --release
   ```

## Dependencies

This project uses the following crates:

| Crate | Version | Description |
|-------|---------|-------------|
| [bevy](https://crates.io/crates/bevy) | 0.18 | Game engine framework |
| [bevy_workbench](https://crates.io/crates/bevy_workbench) | 0.2 | Editor scaffold with dock layout, inspector, and console |
| [bevy_ecs_tiled](https://crates.io/crates/bevy_ecs_tiled) | 0.11 | Tiled map loading and rendering via ECS |

## Contributing

Contributions are welcome!
Whether you want to fix a bug, add a feature, or improve documentation:

* Submit an **Issue** or **Pull Request**.
* Share ideas and discuss design or architecture.

## License

This project is licensed under the GNU General Public License v3.0 or later — see the [LICENSE](LICENSE) file for details.
