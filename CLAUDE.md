# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **AGENTS.md 是本仓库面向 AI 代理的权威文档**（中文，内容详尽：依赖清单、完整目录结构、CI/发布流程、安全注意事项、已知文档不一致等）。本文件只收录最常用的快速参考，细节一律以 AGENTS.md 为准，两处内容冲突时以 AGENTS.md 为准。

## 项目简介

netease-cloud-music-gtk4：基于 **GTK4 + Libadwaita 的 Rust 网易云音乐第三方播放器**（edition 2024，GPL-3.0-or-later），支持 Linux、Windows x64 与 macOS。文档与用户可见字符串以中文为主（gettext，目前仅 `zh_CN`）。

## 常用命令

```bash
# 本地免安装构建并运行（首选方式，debug 构建，资源/GSettings 均可用）
make run

# 全量构建（Meson 生成 src/config.rs 并调用 cargo；默认 release）
meson setup _build && ninja -C _build

# 快速编译检查（注意：需先运行过一次 meson setup 生成 src/config.rs）
cargo build          # 或 cargo check / cargo clippy / cargo fmt

# 数据文件校验（desktop/metainfo/gschema，在构建目录中运行）
ninja -C _build test
```

- Windows MSVC：`build-aux/windows/bootstrap.ps1`（默认前缀 `C:\ncm-gtk`）→ `build.ps1 -Package`；细节见 `build-aux/windows/README.md`。运行便携包目录中的 exe，勿直接打开 `_windows/install/bin` 裸 exe。
- Linux-only MPRIS/ksni/dbus 与 Windows 原生依赖必须按 target 隔离，禁止在 Windows 包中混入 MinGW DLL。
- 日志默认关闭，用 `RUST_LOG=debug` 开启。
- **已有少量 Rust 纯逻辑单元测试**（“我的”页预览/请求代次、播放列表与 Windows 运行时路径）；UI 改动仍主要靠编译通过、手工运行和 Meson 数据校验。

## 架构要点（big picture）

- **单线程 GLib MainContext**：不是 tokio 运行时；用 `MAINCONTEXT.spawn_local()` 派生异步任务。GUI 对象不可跨线程，跨上下文传 `glib::SendWeakRef`。
- **Action 消息总线**：UI 与后端经 `async-channel` 解耦。`src/application.rs` 定义全局 `Action` 枚举（约百种消息）集中分发。新增功能遵循 **GUI 发 Action → Application 处理 → 回发 Action 更新 UI** 的模式。
- **页面导航**：`src/model.rs` 的 `PageStack` 包装 `gtk::Stack` 管理页面栈。
- **UI 构建方式**：所有页面/控件是 `CompositeTemplate` 子类（`src/gui/*.rs`）+ `data/gtk/*.ui` 模板一一对应；资源路径前缀 `/com/gitee/gmg137/NeteaseCloudMusicGtk4/`。
- **集中式样式**：`data/themes/modern.css` 是现代化样式集中地（页面骨架/歌曲行/卡片等），由 `window.rs` 启动时加载。其头注约定：**只允许 Libadwaita 命名色，禁止硬编码颜色值**（图片遮罩/阴影等主题无关场景除外，可用 white/black 关键字）。
- **持久化**：GSettings（主题/代理/音质/歌词等）；用户数据目录下的 `cookies.json`（登录 cookie，敏感，勿入日志/提交）；平台缓存目录；全平台应用内歌词缓存使用 `~/.lyrics`，Linux 外部桌面歌词也复用该目录。

## 改动时的硬性约定

- 新增 `.ui` / `.css` → 登记 `data/netease_cloud_music_gtk4.gresource.xml`。
- 新增 `.rs` → 登记 `src/meson.build` 的 `rust_sources`。
- 用户可见字符串 → `gettextrs::gettext(...)` 包裹 + 登记 `po/POTFILES`。
- `Cargo.lock` 与 `src/config.rs` 被 gitignore，不要提交。
- 版本号三处同步：`Cargo.toml`、根 `meson.build`、Flatpak manifest 的 git tag。
