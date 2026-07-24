# Windows MSVC 构建

Windows 使用与 Linux 相同的 Meson → Cargo → Meson install 构建契约，但依赖引导和便携打包独立。所有原生依赖必须来自同一个 gvsbuild MSVC 前缀，禁止混入 MSYS2/MinGW DLL。

## 前置环境

- Windows 10/11 x64
- Visual Studio 2022 Build Tools（Desktop development with C++）
- Microsoft Visual C++ 2013 Redistributable x64（gvsbuild 的 Perl/OpenSSL 需要 `MSVCR120.dll`）
- MSYS2（仅供 gvsbuild 使用）
- uv、Rustup、Meson、Ninja

在 “Developer PowerShell for VS 2022” 中执行：

```powershell
uv tool install meson
uv tool install ninja

# 首次约需较长时间：会预取易失败的上游源码，再用 gvsbuild 构建
# 默认输出到短路径 C:\ncm-gtk（避免 Desktop 长路径触发 MSVC C1083）
$prefix = .\build-aux\windows\bootstrap.ps1 | Select-Object -Last 1
.\build-aux\windows\build.ps1 -DependencyPrefix $prefix -BuildType debug
```

`bootstrap.ps1` 默认把依赖建在 `C:\ncm-gtk`。仓库内旧的 `_windows\gvsbuild` 若仍存在，脚本会自动迁过去；并会创建 `_windows\gvsbuild` → `C:\ncm-gtk` 联接以兼容旧绝对路径。源码预取（cairo 与 GStreamer 核心/插件包）放在该目录的 `src` 下。中断后可直接重跑：已成功项目会被 `--fast-build` 跳过。MVP 用 `--skip webrtc-audio-processing`（播放不需要 webrtcdsp），并暂时不编 `gst-libav`/`ffmpeg`（可后续再启用）。若报缺少 `gstreamer-play-1.0.pc`，说明 `gst-plugins-bad` 尚未编完，继续重跑 bootstrap 即可。

**日常开发（推荐）**：依赖前缀就绪后，在仓库根目录执行：

```powershell
make dev
# 或等价地：
.\build-aux\windows\dev.ps1
```

`dev.ps1` 会：`build.ps1`（默认 debug）→ 若便携包已存在则只同步 exe/gresource/locale/gschema，否则完整 `package.ps1` → 从 `_windows\dist\...\`（含 DLL）启动。可用 `-NoStart` 只构建不启动，`-Repackage` 强制重打包，`make BUILDTYPE=release dev` 切 release。

**运行注意**：`build.ps1` 不带 `-Package` 时只写入 `_windows\install`；请再执行带 `-Package` 的打包、`make dev`，或打开 `_windows\dist\...` 目录中的 exe。直接双击 `_windows\install\bin\*.exe` 会缺 DLL/gresource。

常见失败对照：

| 现象 | 处理 |
|------|------|
| `MSVCR120.dll` / Perl Configure 闪退 | 安装 VC++ 2013 x64 运行库 |
| `cargo-c` 要求更新的 rustc | 脚本会重置 gvsbuild 私有 rustup 到 stable-msvc |
| `libvpx` tlog 被占用 | 结束残留 `MSBuild`/`cl`/`gvsbuild` 后重跑 |
| `libvpx`：`/tmp/vpx-conf-*.c` 找不到 / `vpxmd.lib` 缺失 | Git Bash 与 MSYS2 抢 PATH（常见于 GitHub `windows-*` runner）。`bootstrap.ps1` 会优先 `C:\msys64\usr\bin` 并传 `--use-env`；本地请确保已装 MSYS2 |
| `webrtc` / abseil `C1083` Invalid argument | 使用默认短路径 `C:\ncm-gtk`，勿把 BuildRoot 放在 Desktop 深目录 |

生成 release 便携包：

```powershell
.\build-aux\windows\build.ps1 `
  -DependencyPrefix $prefix `
  -BuildType release `
  -Package
```

产物位于 `_windows\dist\netease-cloud-music-gtk4-<版本>-windows-x64.zip`。

## 目录与平台隔离

- `C:\ncm-gtk`：默认 gvsbuild MSVC 依赖前缀（GTK4、Libadwaita、GStreamer、gettext、OpenSSL）。
- `_windows\build`：Meson 与 Cargo Windows 构建目录。
- `_windows\install`：平台无关的 Meson install 树。
- `_windows\dist`：Windows 便携包。
- Linux 继续使用 `_build`/`_local` 和既有 deb、rpm、AppImage 流程。

Windows 打包器会检查 DLL 导入；发现 `libgcc`、`libwinpthread`、`libstdc++` 或 MSYS runtime 时直接失败。运行时只加载包内 schema、图标、pixbuf loader 与 GStreamer 插件，避免和用户机器上的其他 GTK/GStreamer 安装互相干扰。
