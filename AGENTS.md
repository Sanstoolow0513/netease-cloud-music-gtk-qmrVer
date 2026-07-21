# AGENTS.md

> 本文件面向 AI 编码代理，介绍本项目的架构、构建流程与开发约定。阅读前无需任何项目背景知识。

## 项目概览

**netease-cloud-music-gtk4** 是基于 **GTK4 + Libadwaita** 的网易云音乐第三方桌面播放器，使用 **Rust**（edition 2024）编写，支持 Linux、Windows x64 与 macOS。Linux 在 openSUSE Tumbleweed + GNOME 下测试；Windows 使用 MSVC 便携包。当前版本 **2.5.3**，许可证 GPL-3.0-or-later。

- 应用 ID：`com.gitee.gmg137.NeteaseCloudMusicGtk4`
- 风格仿 GNOME Music，支持发现页、榜单、歌单、搜索、"我的"页、播放栏、应用内歌词、扫码/验证码登录等。
- **Linux 专属桌面集成**：MPRIS2、ksni 托盘、外部桌面歌词（osdlyrics / desktop-lyric）。Windows 首版不提供这些能力，关闭窗口即退出。
- 上游仓库：https://github.com/gmg137/netease-cloud-music-gtk（gitee 镜像同名）。
- 文档（README、Issue 模板）以中文为主；代码注释中英混用，用户可见字符串通过 gettext 翻译（目前仅提供中文 `zh_CN`）。

## 技术栈与关键依赖

- **UI**：gtk4 (gtk-rs 0.11，`v4_10` feature)、libadwaita 0.9（`v1_6` feature）。UI 用 `.ui` 模板（`data/gtk/*.ui`）+ `CompositeTemplate` 子类化方式构建。
- **音频播放**：GStreamer（`gstreamer` / `gstreamer-play` 0.25）。
- **网络 API**：`ncm-api` crate（`netease-cloud-music-api`，锁定 gitee 仓库 tag 2.0.0；GitHub dev 分支以注释形式保留在 `Cargo.toml` 中）。
- **桌面集成**：`mpris-server`（MPRIS2 D-Bus 接口）、`ksni`（系统托盘）、`qrcode-generator`（扫码登录）。
- **其他**：`async-channel`（内部消息分发）、`once_cell`、`gettext-rs`（i18n）、`cookie_store`（登录 cookie 持久化）、`serde/serde_json`、`anyhow`、`regex`、`chrono`、`fastrand`、`log` + `env_logger`。
- 系统依赖（见根 `meson.build`）：openssl、glib-2.0/gio-2.0 (≥2.66)、gdk-pixbuf、gtk4 (≥4.10)、libadwaita-1 (≥1.5)、gstreamer 1.0 系列 (≥1.16，含 base/bad/plugins)；dbus-1、MPRIS 与 ksni 仅用于 Linux。Windows 原生依赖由单一 gvsbuild MSVC 前缀提供。

## 构建与运行

项目使用 **Meson + Cargo** 双重构建：Meson 负责安装数据文件、编译 gresource、生成 `src/config.rs`（由 `src/config.rs.in` 生成，提供 `VERSION`/`PKGDATADIR`/`LOCALEDIR`/`GETTEXT_PACKAGE`），并通过平台包装器调用 `cargo build`：Linux/macOS 使用 `build-aux/cargo.sh`，Windows 使用 `build-aux/windows/cargo.ps1`。

```bash
# 编译
meson setup _build
cd _build && ninja

# 安装（需要 root）
sudo ninja install

# 运行构建出的二进制（Linux/macOS 未安装时 gresource 可能失败：
# 非 Windows 仍按 meson 生成的绝对 PKGDATADIR 加载；Windows 便携包则相对 exe 解析）
./_build/src/netease-cloud-music-gtk4
```

也可以直接用 Cargo 编译调试：

```bash
# 注意：直接 cargo 构建不会生成 src/config.rs，
# 需先运行一次 meson setup（config.rs 被 .gitignore 忽略，不入库）
cargo build
cargo run
```

**本地免安装运行**：根目录 `Makefile` 提供了无需 root、不写入系统目录的运行方式。它使用独立构建目录 `_local`，将 meson `prefix` 指向项目内 `_local/prefix`（使编译期写入 `src/config.rs` 的 `PKGDATADIR`/`LOCALEDIR` 均为本地路径），`meson install` 到该本地前缀后直接运行（GSettings schema 通过 `GSETTINGS_SCHEMA_DIR` 指向本地编译产物）。

```bash
make run     # 构建并运行（默认 debug 构建，可用 make BUILDTYPE=release 覆盖）
make build   # 只构建并安装到 _local/prefix
make clean   # 删除 _local
```

Windows 在 VS 2022 Developer PowerShell 中构建；详细依赖和命令见 `build-aux/windows/README.md`：

```powershell
$prefix = .\build-aux\windows\bootstrap.ps1 | Select-Object -Last 1
.\build-aux\windows\build.ps1 -DependencyPrefix $prefix -BuildType release -Package
```

- 依赖默认建在短路径 `C:\ncm-gtk`（避免 Desktop 长路径触发 MSVC `C1083`）；仓库内 `_windows\gvsbuild` 可为指向该前缀的联接。
- 只允许 `x86_64-pc-windows-msvc` 与同一 gvsbuild 前缀，禁止混入 MinGW/MSYS2 DLL。
- **运行**：使用 `_windows\dist\netease-cloud-music-gtk4-<ver>-windows-x64\`（或对应 zip）中的 exe。Meson install 树 `_windows\install\bin\` 的裸 exe 缺 DLL/资源，不能直接双击。
- 便携包运行时从 exe 相对目录加载 gresource、locale、schema、图标和 GStreamer 插件；`src/platform/mod.rs` 在 Windows 上于 `gstreamer::init` 前设置相关环境变量。
- MVP bootstrap 跳过 `webrtc-audio-processing`，暂不编 `gst-libav`/`ffmpeg`（解码依赖 good/ugly 等插件；缺格式时可再启用）。

查看日志：从终端启动并设置环境变量 `RUST_LOG=debug` 或 `RUST_LOG=netease_cloud_music_gtk4`（默认日志级别为 off，见 `src/main.rs`）。

macOS 构建时根目录 `build.rs`（Cargo 自动识别）会设置 GStreamer framework 的 pkg-config / rpath 路径；`Cargo.toml` 中 `[package.metadata.bundle]` 供 cargo-bundle 打包 macOS dmg 使用。仓库根部的 `.buildconfig` 是 GNOME Builder 的配置文件，与构建脚本无关。

## 测试

- **Rust 代码目前只有少量内联单元测试**（`application.rs` / `model.rs` 的“我的”页预览与请求代次；`playlist.rs` 与 `platform/mod.rs` 另有少量路径相关测试；`Cargo.toml` 仍无 `[dev-dependencies]`），CI 尚未配置 `cargo test`。UI 改动仍主要依靠编译通过 + 手工运行验证。
- Meson 层面定义了数据文件校验测试（在 `_build` 中运行 `meson test` / `ninja test`，见 `data/meson.build`）：
  - `desktop-file-validate` 校验 desktop 文件
  - `appstreamcli validate` 校验 metainfo
  - `glib-compile-schemas --strict --dry-run` 校验 gschema
- CI（见下文）以能否完整构建为事实上的回归检查。提交前至少应确保 `cargo build`（或 meson 全量构建）无警告错误，且相关 `.ui` / gschema 变更通过上述校验。

## 代码结构

```
src/
├── main.rs          # 入口：初始化日志/gstreamer/路径/gettext/gresource，启动 Application
├── application.rs   # NeteaseCloudMusicGtk4Application：全局 Action 事件循环与分发（~1500 行核心）
├── window.rs        # 主窗口（CompositeTemplate，绑定 gtk/window.ui），页面栈与全局状态（~1200 行）
├── model.rs         # 共享数据结构：UserInfo、PageStack（页面导航栈）、图片加载工具等
├── ncmapi.rs        # NcmClient：封装 ncm-api MusicApi，cookie 持久化、音质/码率映射
├── path.rs          # DATA/CONFIG/CACHE；全平台歌词缓存 LYRICS（~/.lyrics）
├── platform/mod.rs  # 托盘/外部歌词能力；Windows 相对 exe 的运行时路径
├── utils.rs         # 工具函数
├── config.rs.in     # Meson 生成 config.rs 的模板
├── audio/
│   ├── mod.rs       # 按 target 选用 mpris 或 mpris_stub
│   ├── playlist.rs  # 播放列表与 LoopsState（不依赖 mpris_server 类型）
│   ├── mpris.rs     # Linux：MprisController（MPRIS2）
│   └── mpris_stub.rs # 非 Linux：no-op
└── gui/             # 各页面/控件，均为 CompositeTemplate 子类 + data/gtk/*.ui
    ├── discover.rs  # 发现页（轮播、推荐歌单、新专辑）
    ├── toplist.rs   # 榜单页
    ├── my_page.rs   # 我的页
    ├── player_controls.rs   # 底部播放栏（~1300 行，播放控制核心 UI）
    ├── playlist_lyrics.rs   # 播放列表 + 歌词页
    ├── search_song_page.rs / search_songlist_page.rs / search_singer_page.rs  # 搜索页
    ├── songlist_page.rs / songlist_view.rs / songlist_row.rs / songlist_grid_item.rs  # 歌单相关组件
    ├── preferences.rs       # 首选项；按平台能力隐藏托盘/桌面歌词相关项
    ├── user_menus.rs        # 用户菜单/登录（二维码、验证码）
    ├── system_tray.rs       # Linux：ksni 托盘
    ├── system_tray_stub.rs  # 非 Linux：no-op
    └── theme_selector.rs    # 主题切换组件

data/
├── gtk/*.ui                 # GTK Builder 模板（与 gui 模块一一对应）
├── themes/*.css             # 自定义样式；modern.css 为集中式现代化样式（页面骨架/歌曲行/卡片/详情页头部/发现页），由 window.rs 在启动时按资源路径加载
├── icons/hicolor/           # 应用图标
├── *.gschema.xml            # GSettings 模式（com.gitee.gmg137.NeteaseCloudMusicGtk4）
├── *.desktop.in / *.metainfo.xml.in   # 桌面文件与 AppStream 元数据模板
├── meson.build              # 编译 gresource、安装/校验数据文件
└── netease_cloud_music_gtk4.gresource.xml  # 资源清单（新增 .ui/.css 需登记）

po/                          # gettext 翻译（POTFILES 登记需翻译的源文件，目前仅 zh_CN）
build-aux/cargo.sh           # Linux Meson 调用 cargo 的包装脚本
build-aux/windows/           # Windows MSVC 依赖、构建和便携打包脚本
build.rs                     # macOS GStreamer framework 路径设置
com.gitee.gmg137.NeteaseCloudMusicGtk4.json  # Flatpak manifest（GNOME Platform 45）
```

### 运行时架构要点

- **单线程 GLib MainContext 架构**：应用不是多线程 tokio 运行时，而是基于 GLib 主循环。全局 `MAINCONTEXT`（`main.rs` 中的 `Lazy<glib::MainContext>`）用于 `spawn_local` 派生异步任务。
- **Action 消息总线**：UI 与后端通过 `async-channel` 解耦。`application.rs` 定义了庞大的 `Action` 枚举（播放、登录、页面路由、发现页、榜单、歌词等约百种消息）和 `ActionCallback` 回调类型；各 GUI 组件持有 `Sender<Action>` 发送请求，Application 集中处理后再通过 Action 回投结果。新增功能时遵循"GUI 发 Action → Application 处理 → 回发 Action 更新 UI"的模式。
- **页面导航**：`model.rs` 的 `PageStack` 包装 `gtk::Stack`，管理页面 push/pop/切换与延迟移除。
- **持久化**：
  - GSettings（schema `com.gitee.gmg137.NeteaseCloudMusicGtk4`）：主题、循环模式、代理、音质、缓存清理、音量、桌面歌词等。
  - 文件系统：GLib 用户缓存/数据目录下的 `netease-cloud-music-gtk4`（Linux 常见为 `~/.cache` / `~/.local/share`，Windows 为 AppData 对应路径）；登录 cookie `cookies.json`（见 `ncmapi.rs`）；全平台应用内歌词缓存使用 `~/.lyrics`，Linux 外部桌面歌词也复用该目录。
- **MPRIS 名称**（仅 Linux）：`org.mpris.MediaPlayer2.NeteaseCloudMusicGtk4`。
- **平台隔离**：`mpris-server`/`ksni` 仅 `cfg(target_os = "linux")`；非 Linux stub 负责保持相同 API 形状，Action 消息不按平台拆分。`platform::HAS_*` 只用于设置显隐、关窗行为和外部桌面歌词等确实不同的用户行为；可由 stub 吸收的 MPRIS/托盘调用保持统一路径。

## 代码风格与约定

- 标准 Rust 风格，4 空格缩进（rustfmt 默认）；未提供自定义 rustfmt/clippy 配置，提交前可运行 `cargo fmt` 与 `cargo clippy`。
- 源文件头部惯例（多数文件）带有版权注释块：
  ```rust
  //
  // xxx.rs
  // Copyright (C) 2022 gmg137 <gmg137 AT live.com>
  // Distributed under terms of the GPL-3.0-or-later license.
  //
  ```
- GTK 代码遵循 gtk-rs 惯例：`mod imp { ... }` 内部结构体 + `glib::wrapper!`、`CompositeTemplate` 绑定 `.ui` 资源路径 `/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/xxx.ui`、`glib::clone!` 宏处理闭包捕获。
- 单线程约束：GUI 对象不可跨线程，跨上下文传递用 `glib::SendWeakRef`；`MprisController` 上有显式的 `unsafe impl Send/Sync` 注释。
- 用户可见字符串使用 `gettextrs::gettext(...)` 包裹，并把源文件加入 `po/POTFILES`。
- 新增 `.ui` / `.css` 文件时：放入 `data/gtk/` 或 `data/themes/`，登记到 `data/netease_cloud_music_gtk4.gresource.xml`；新增 `.rs` 文件需登记到 `src/meson.build` 的 `rust_sources`。
- 依赖版本统一用 `~x.y` 形式写在 `Cargo.toml`；系统库版本约束在根 `meson.build` 中声明，两者需保持同步。
- `Cargo.lock` 与 `src/config.rs` 均被 gitignore（另有 `/target`、`/build`、`/_local`、`/_windows`、`/worktrees`），不要提交。
- Windows 构建细节以 [`build-aux/windows/README.md`](build-aux/windows/README.md) 为准；`CLAUDE.md` 仅作快速索引，冲突时以本文件为准。

### 已知注意点（改动时注意）

- Flatpak manifest、AppStream、桌面文件仍以 Linux 分发为主；Windows 不安装 `.desktop`/AppStream。
- Windows 便携包与 GitHub Release 附件：需本分支合入并走 `release.yml`/`nightly.yml` 后才会出现在正式 Release；本地产物在 `_windows/dist/`。依赖前缀就绪不等于应用已打包：须再跑 `build.ps1 -Package`。
- `docs/superpowers/` 下的 dated plans/specs 是历史设计记录，不作为现役构建/平台能力合同。

## 发布与部署流程

- **版本号三处同步**：`Cargo.toml` 的 `version`、根 `meson.build` 的 `project(version)`、Flatpak manifest 中的 git `tag`。
- **CI**（`.github/workflows/` + 本地 composite actions）：
  - `meson.yml`：push/PR 到 master 时分别执行 Linux 与 Windows MSVC 构建；Linux 打包 AppImage，Windows 打包便携 zip。
  - `nightly.yml`：每日定时检查变更后触发 nightly 构建。
  - `release.yml`：推送 `x.y.z` 格式 tag 触发。Linux 构建 .deb / .rpm / AppImage，macOS（Intel + ARM）构建 dmg，Windows 构建 x64 便携 zip，最后汇总创建 GitHub Release。
- **分发渠道**：openSUSE (zypper)、Arch AUR/archlinuxcn、Ubuntu PPA (`ppa:gmg137/ncm`)、Debian 中文社区源、Flathub Flatpak、Nix、Gentoo gentoo-zh 源——这些包由各渠道维护；仓库直接产出 AppImage/deb/rpm/dmg，以及（本分支合入并走 release/nightly 后）Windows zip。现役 GitHub Release `2.5.3` 目前仅有 AppImage。

## 安全注意事项

- 登录 cookie 以 JSON 存于用户数据目录（`~/.local/share/netease-cloud-music-gtk4/cookies.json`），属于敏感数据；调试或提交日志时不要泄露其内容。
- 网络请求通过 `ncm-api` 访问网易官方 API（`ncmapi.rs` 中的 `BASE_URL_LIST` 用于客户端日志/反馈上报地址过滤）；支持用户配置 HTTP 代理（GSettings `proxy-address`）。
- Flatpak 沙箱权限见 manifest：网络、Wayland/X11、PulseAudio、DRI、`~/.lyrics` 目录访问、MPRIS D-Bus 名称。新增权限需求时需同步修改 manifest 并说明理由。
- 不要把任何凭据、token 写入代码或日志；日志默认关闭（`RUST_LOG` 显式开启），注意 debug 日志可能包含 API 响应内容。
