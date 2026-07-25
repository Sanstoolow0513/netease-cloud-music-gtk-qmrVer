# NeteaseCloudMusicGtk4（qmrVer）

基于 **GTK4 + Libadwaita** 的网易云音乐第三方桌面播放器，用 **Rust** 编写。本仓库是上游 [gmg137/netease-cloud-music-gtk](https://github.com/gmg137/netease-cloud-music-gtk) 的 fork，在保留原有播放与桌面集成能力的同时，加强跨平台构建与桌面优先的自适应界面。

| 平台 | 状态 |
|------|------|
| Linux | 在 openSUSE Tumbleweed + GNOME 下测试；提供 MPRIS2、ksni 托盘、外部桌面歌词 |
| Windows 10/11 x64 | MSVC 便携包；关闭窗口即退出，无托盘 / MPRIS / 外部桌面歌词 |
| macOS | 可构建；能力与 Linux 桌面集成项按平台门控裁剪 |

当前版本：**2.5.3** · 许可证：[GPL-3.0-or-later](COPYING)

## 特点

- **原生 GTK**：仿 GNOME Music 风格，Libadwaita 明暗主题与系统风格一致。
- **跨平台核心**：同一套播放、登录与页面逻辑；Linux 专属桌面集成与 Windows 打包流程相互隔离。
- **桌面优先自适应**：按窗口宽度分档（窄 / 标准 / 宽），导航、内容网格与播放栏随档切换；支持窗口几何记忆与 F11 全屏。
- **轻量分发**：Linux 可用系统运行时；Windows 便携包自带 GTK / GStreamer，无需另行安装。
- **Rust 性能**：相对 Node / Python 实现更轻、更顺滑。

## 功能

- 发现页、榜单、歌单详情、搜索、我的页（内容分区预览）
- 扫码 / 验证码登录、网络代理、音质与缓存策略
- 播放栏、播放列表、应用内歌词；可配置顶栏页面顺序与显隐
- 首选项：主题、循环模式、代理、音质等
- **Linux**：MPRIS2、系统托盘、外部桌面歌词（[desktop-lyric](https://github.com/tuberry/desktop-lyric) 或 osdlyrics）

## 截图

![](./screenshots/discover.png)
![](./screenshots/discover-dark.png)
![](./screenshots/toplist.png)

## 安装

### Windows 10/11 x64

从本仓库 [GitHub Releases](https://github.com/Sanstoolow0513/netease-cloud-music-gtk-qmrVer/releases) 下载 `netease-cloud-music-gtk4-<版本>-windows-x64.zip`（若该版本尚未附带 Windows 包，可用本地构建产物）。

解压后运行目录内的 `netease-cloud-music-gtk4.exe`。**不要**直接运行 Meson install 树（如 `_windows\install\bin\`）里的裸 exe，否则会缺 DLL 与资源。

本地 MSVC 构建见 [build-aux/windows/README.md](build-aux/windows/README.md)。

### Linux 发行版包

下列渠道多由维护者跟踪**上游**包名；行为与本 fork 不完全一致时，请以本仓库源码 / Release 为准。

<details>
<summary>openSUSE / Arch / Ubuntu / Debian / Flatpak / Nix / Gentoo</summary>

#### openSUSE Tumbleweed

```bash
sudo zypper in netease-cloud-music-gtk
```

#### openSUSE Leap

```bash
sudo zypper ar -f obs://multimedia:apps multimedia
sudo zypper in netease-cloud-music-gtk
```

#### Arch Linux

```bash
# AUR
paru -S netease-cloud-music-gtk4
# archlinuxcn
sudo pacman -Syu netease-cloud-music-gtk4
```

#### Ubuntu（24.10 / 24.04 / 22.04）

```bash
sudo add-apt-repository ppa:gmg137/ncm
sudo apt update
sudo apt install netease-cloud-music-gtk
```

#### Debian

先按 [Debian 中文社区软件源](https://github.com/debiancn/repo/blob/master/README.rst) 添加源，再：

```bash
sudo apt install netease-cloud-music-gtk
```

#### Flatpak（Flathub）

<a href='https://flathub.org/apps/com.github.gmg137.netease-cloud-music-gtk'>
    <img width='240' alt='Download on Flathub' src='https://flathub.org/api/badge?locale=zh-Hans'/>
</a>

离线安装示例：

```bash
sudo flatpak install com.gitee.gmg137.NeteaseCloudMusicGtk4-*.flatpak
```

#### Nix

```bash
nix-env -iA nixpkgs.netease-cloud-music-gtk
```

#### Gentoo

```bash
sudo emerge --ask app-eselect/eselect-repository
sudo eselect repository enable gentoo-zh
sudo emerge --sync gentoo-zh
sudo emerge --ask media-sound/netease-cloud-music-gtk
```

</details>

### 从本仓库源码构建

**运行依赖（Linux）**：openssl、gstreamer 及 plugins-base / good / bad / ugly。

**编译依赖（Linux）**：openssl、dbus、gtk4（≥4.10）、gdk-pixbuf、libadwaita-1（≥1.5）、gstreamer（≥1.16，含 base / audio / play）。Windows 见上文 MSVC 说明（不依赖 dbus 包）。

#### Linux / macOS（推荐本地免安装）

```bash
git clone https://github.com/Sanstoolow0513/netease-cloud-music-gtk-qmrVer.git
cd netease-cloud-music-gtk-qmrVer

make run          # debug 构建并运行（写入项目内 _local，无需 root）
# make BUILDTYPE=release run
# make clean      # 删除 _local
```

系统安装（需 root）：

```bash
meson setup _build
ninja -C _build
sudo ninja -C _build install
```

#### Windows MSVC

在 VS 2022 Developer PowerShell 中（首次需引导依赖）：

```powershell
$prefix = .\build-aux\windows\bootstrap.ps1 | Select-Object -Last 1

# 日常开发：构建并启动带 DLL 的便携包（默认 debug）
make dev
# 等价：.\build-aux\windows\dev.ps1

# 正式便携包（含 zip）
.\build-aux\windows\build.ps1 -DependencyPrefix $prefix -BuildType release -Package
```

运行目录为 `_windows\dist\netease-cloud-music-gtk4-<版本>-windows-x64\`（或对应 zip）。细节与排错见 [build-aux/windows/README.md](build-aux/windows/README.md)。

## 快捷键

| 快捷键 | 作用 |
|--------|------|
| `Ctrl+F` / `/` | 搜索 |
| `Ctrl+Backspace` / `Esc` | 返回 |
| `F11` | 全屏切换 |
| `Ctrl+Q` | 退出 |

## FAQ

1. **为什么后台没有托盘图标？**  
   Linux 通过 ksni 提供托盘；Windows / 无托盘平台关闭窗口即退出。

2. **Windows 上双击 exe 提示缺少 DLL？**  
   请运行 `_windows\dist\...`（或 Release zip 解压目录）里的 exe，而不是 `_windows\install\bin\` 下的裸文件。

3. **使用 osdlyrics 时歌词匹配不对？**  
   在 osdlyrics 首选项 → 歌词位置 → 文件名中添加规则：`%t-%p-%a`。应用内与外部桌面歌词缓存目录均为 `~/.lyrics`。

4. **音乐缓存目录在哪？**  
   Linux 通常为 `~/.cache/netease-cloud-music-gtk4`。Windows 上 GLib 用户缓存目录指向 `AppData\Local\Microsoft\Windows\INetCache`（不是 `AppData\Local`），排查图片缓存时注意路径。

5. **如何分享歌曲？**  
   点击播放栏歌曲名称，会复制歌曲链接等信息到剪贴板。

6. **如何查看日志？**  
   从终端启动并设置 `RUST_LOG=debug` 或 `RUST_LOG=netease_cloud_music_gtk4`（默认日志关闭）。

## 开发说明

面向贡献者与 AI 代理的架构、约定与红线见 [AGENTS.md](AGENTS.md)（权威）与 [CLAUDE.md](CLAUDE.md)（速查）。UI 自适应设计背景见 [docs/ui-redesign-2026-07.md](docs/ui-redesign-2026-07.md)。

协作默认针对本 fork：`Sanstoolow0513/netease-cloud-music-gtk-qmrVer`。

## License

源码与文档遵循 [GNU General Public License v3](COPYING)（GPL-3.0-or-later）。

## 参考

- [上游仓库](https://github.com/gmg137/netease-cloud-music-gtk)
- [Shortwave](https://gitlab.gnome.org/World/Shortwave)
- [gnome-music](https://gitlab.gnome.org/GNOME/gnome-music)
