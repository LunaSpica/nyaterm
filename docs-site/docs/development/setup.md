---
sidebar_position: 2
---

# 开发环境搭建

NyaTerm 应用本身是 Cargo workspace。Node.js 和 pnpm 只用于构建本仓库中的 Docusaurus 文档站点。

## 应用开发前置要求

### Rust 与 Git

安装最新稳定版 Rust：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Windows 用户可从 [rustup.rs](https://rustup.rs/) 安装。所有平台还需要 Git 和目标平台的原生编译工具链。

### 平台依赖

#### Windows

安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，并选择“使用 C++ 的桌面开发”工作负载。

#### macOS

安装 Xcode 和命令行工具：

```bash
xcode-select --install
```

GPUI 在 macOS 上使用 Metal，因此需要可用的 macOS SDK。

#### Linux（Ubuntu / Debian）

安装 Rust crate 编译工具、字体/窗口系统开发库和 Vulkan loader：

```bash
sudo apt update
sudo apt install build-essential pkg-config clang cmake libfontconfig-dev \
  libglib2.0-dev libssl-dev libvulkan1 libwayland-dev libx11-xcb-dev \
  libxkbcommon-x11-dev
```

运行桌面应用还需要可用的 Vulkan 驱动，以及 X11 或 Wayland 会话。

## 获取源码

```bash
git clone https://github.com/nyakang/nyaterm.git
cd nyaterm
```

应用依赖由 Cargo 管理，不需要安装根目录的 JavaScript 依赖。

## 启动应用

```bash
cargo run -p nyaterm-app --bin nyaterm
```

首次编译会构建 GPUI 和 vendored dependencies，耗时会明显长于后续增量构建。

## 常用检查

迭代时优先运行受影响 crate 的检查：

```bash
cargo check -p nyaterm-app
cargo test -p <crate-name>
```

提交评审前运行相关 workspace 检查：

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

`cargo fmt --all` 会写回格式化结果，仅在准备应用格式变更时运行。

## Release profile 构建

```bash
cargo build -p nyaterm-app --bin nyaterm --release
```

原生二进制位于 `target/release/nyaterm`，Windows 下为 `target/release/nyaterm.exe`。该命令只构建应用二进制，不生成 `.msi`、`.dmg`、`.deb` 或 AppImage 安装包。

## 文档站开发

编辑 `docs-site` 时另外安装 Node.js 18+ 和 [pnpm](https://pnpm.io/)：

```bash
pnpm --dir docs-site install
pnpm --dir docs-site start:zh
```

英文文档开发服务器：

```bash
pnpm --dir docs-site start:en
```

构建全部 locale：

```bash
pnpm --dir docs-site build
```

文档构建会检查页面、sidebar、Markdown 链接和中英文翻译资源。

## 开发约定

- 先阅读根目录 `AGENTS.md` 和 `CONTRIBUTING.md`。
- UI 状态与视图放在 `nyaterm-desktop`，共享控件放在 `nyaterm-ui`。
- transport、terminal 和 core crate 保持独立于 GPUI。
- 新增 UI 文本时同步更新 `crates/nyaterm-desktop/src/i18n/locales/` 下的中英文文件。
- 不要在测试、日志或诊断数据中使用真实凭据。
