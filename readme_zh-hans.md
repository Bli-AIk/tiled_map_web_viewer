# tiled_map_web_viewer

[![license](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE) <img src="https://img.shields.io/github/repo-size/Bli-AIk/tiled_map_web_viewer.svg"/> <img src="https://img.shields.io/github/last-commit/Bli-AIk/tiled_map_web_viewer.svg"/> <br>
<img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" /> <img src="https://img.shields.io/badge/Bevy-232326?style=for-the-badge&logo=bevy&logoColor=white" />

> 当前状态：✅ 已可用于地图库预览工作流，但大型 World 加载体验和项目级集成仍在持续打磨。

**tiled_map_web_viewer** — 面向开源地图库项目的 Web 端 Tiled 地图浏览器，可通过 GitHub Pages 在线预览地图集合。

| English                | 简体中文 |
|------------------------|------|
| [English](./readme.md) | 简体中文 |

## 简介

`tiled_map_web_viewer` 是一个基于 Bevy 和 WebAssembly 构建的轻量级 Tiled 地图浏览器。  
它专为开源游戏地图库的维护者设计，让他们能够直接在 GitHub Pages 上提供 `.tmx` 与 `.world` 地图集合的交互式 Web 预览。

使用 `tiled_map_web_viewer`，你只需要将 WASM 构建产物与地图资源一起部署，即可为贡献者和用户提供可视化的地图浏览器——无需安装桌面应用。

仓库中还包含一组来源于 [The Mana World client data](https://github.com/themanaworld/tmwa-client-data) 的示例资源。相关来源与许可证说明见 [assets/NOTICE](assets/NOTICE)。

## 为什么选择 Tiled？

[Tiled](https://www.mapeditor.org/) 是一款免费、开源的 2D 地图编辑器，已成为独立游戏和开源游戏开发社区的事实标准。其 `.tmx` 地图格式几乎被所有主流 2D 游戏引擎和框架所支持——包括 Godot、Unity、Phaser、LÖVE、pygame、libGDX 等等。

选择 Tiled 作为基础格式，`tiled_map_web_viewer` 能够**适配不同游戏社区所使用的多样化创作工具链**。无论你的贡献者使用 Godot、RPG Maker（通过 TMX 导出）还是 Tiled 本身，只要地图能导出为 `.tmx` 格式，就可以在这里预览。这使得它成为服务于多引擎、多工作流的开源地图库项目的理想工具。

## 功能特性

* **地图列表面板** — 浏览库中所有可用地图，点击即可预览
* **Tiled 地图渲染** — 基于 `bevy_ecs_tiled`，支持正交、等距和六角地图
* **编辑器风格 UI** — 基于 `bevy_workbench` 的 Dock 面板布局
* **图层可见性控制** — 单独切换各地图图层的显示/隐藏
* **相机控制** — 缩放和平移以探索地图
* **桌面版 & WASM** — 原生运行或部署到 GitHub Pages
* **世界地图支持** — 加载 `.world` 文件并预览多地图拼接布局
* **渲染设置 Dock** — 调整预览背景、预览网格和 World 描边叠加层

## 当前支持情况

### 已支持

* 独立 `.tmx` 地图
* 使用 `.world` 组织的多地图拼接布局
* 基于结构化 manifest 的地图列表
* 在没有结构化 manifest 时，基于路径的回退发现
* 在示例浏览器中将普通地图与 World 分组显示
* 原生桌面预览与 WebAssembly 部署
* 预览背景色、预览网格、World 描边的渲染控制

### 当前限制

* 大型 `.world` 文件在 WASM 下打开时仍可能需要明显等待时间
* 本 crate 只提供通用浏览器能力；项目专属元数据、标签与工作流仍需由宿主项目提供
* 示例浏览器刻意保持简洁，不能替代 Tiled 编辑器本身

## 使用方法

1. **安装 Rust**（如果尚未安装）：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **克隆仓库**：
   ```bash
   git clone https://github.com/Bli-AIk/tiled_map_web_viewer.git
   cd tiled_map_web_viewer
   ```

3. **运行（桌面版）**：
   ```bash
   cargo run
   ```

4. **运行（WASM）**：
   ```bash
   cargo install trunk
   rustup target add wasm32-unknown-unknown
   trunk serve
   ```

## 构建方法

### 前置要求

* Rust 1.85 或更高版本
* Bevy 0.18 兼容的系统依赖：
  ```bash
  # Linux (Ubuntu/Debian)
  sudo apt-get install -y g++ pkg-config libx11-dev libasound2-dev libudev-dev \
      libwayland-dev libxkbcommon-dev
  ```

### 构建步骤

1. **克隆仓库**：
   ```bash
   git clone https://github.com/Bli-AIk/tiled_map_web_viewer.git
   cd tiled_map_web_viewer
   ```

2. **构建项目**：
   ```bash
   cargo build --release
   ```

3. **构建 WASM 版本**：
   ```bash
   trunk build --release
   ```

## 依赖

本项目使用以下 crate：

| Crate | 版本 | 描述 |
|-------|------|------|
| [bevy](https://crates.io/crates/bevy) | 0.18 | 游戏引擎框架 |
| [bevy_workbench](https://crates.io/crates/bevy_workbench) | 0.3 | 编辑器脚手架，提供 Dock 布局、检查器和控制台 |
| [bevy_ecs_tiled](https://crates.io/crates/bevy_ecs_tiled) | 0.11 | 基于 ECS 的 Tiled 地图加载与渲染 |

## 贡献

欢迎贡献！
无论是修复 bug、添加功能还是改进文档：

* 提交 **Issue** 或 **Pull Request**。
* 分享想法，讨论设计或架构。

## 许可证

本项目使用 GNU 通用公共许可证 v3.0 或更高版本 — 详见 [LICENSE](LICENSE) 文件。
