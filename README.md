# NeteaseCloudMusicGtk4
> netease-cloud-music-gtk4 是基于 GTK4 + Libadwaita 构造的跨平台网易云音乐播放器。Linux 版在 openSUSE Tumbleweed + GNOME 下测试；Windows 版面向 Windows 10/11 x64。

## 特点
- 跨平台：共享同一套播放与界面核心，Linux 和 Windows 使用相互隔离的平台集成与打包流程。
- 极速：相比 Node/python 版，Rust + GTK 带给你如丝般的顺滑体验。
- 可靠：除了断网或网易 API 限制，不会出现运行时问题。
- 简洁：仿 GNOME Music 风格，GTK 原生界面，纯粹得令人发指。
- 轻量：Linux 可使用系统运行时；Windows 便携包内含 GTK/GStreamer，无需另行安装。

## 路线图
- [x] 发现页
- [x] 榜单页
- [x] 歌单详情页
- [x] 自适应皮肤
- [x] 网络代理
- [x] 扫码登录
- [x] 验证码登录
- [x] 播放栏
- [x] 多语言支持
- [x] 歌单页
- [x] 搜索页
- [x] 我的页
- [x] 首选项
- [x] Mpris2 绑定
- [x] 播放列表
- [x] 歌词
- [X] 桌面歌词(依赖于 [desktop-lyrics](https://github.com/tuberry/desktop-lyric) 或 osdlyrics)

## Linux 运行依赖
> openssl, gstreamer, gstreamer-plugins-base, gstreamer-plugins-good, gstreamer-plugins-bad, gstreamer-plugins-ugly

## 安装
### Windows 10/11 x64
本地/CI 构建后，使用便携目录中的 `netease-cloud-music-gtk4.exe`（不要直接运行 Meson install 树里的 `bin\` 裸 exe，否则会缺 DLL 和资源）。正式发布后可从 GitHub Release 下载 `netease-cloud-music-gtk4-<版本>-windows-x64.zip`；当前已发布的 `2.5.3` 尚无 Windows 附件。

Windows 首版支持登录、搜索、播放、缓存和应用内歌词；暂不支持 Linux MPRIS、ksni 托盘、“退出到后台”和依赖 osdlyrics 的外部桌面歌词。MSVC 构建说明见 [build-aux/windows/README.md](build-aux/windows/README.md)。

### openSUSE Tumbleweed
```bash
sudo zypper in netease-cloud-music-gtk
```
### openSUSE Leap
```bash
// 添加源
sudo zypper ar -f obs://multimedia:apps multimedia
// 安装
sudo zypper in netease-cloud-music-gtk
```

### Arch Linux
```bash
# AUR
paru -S netease-cloud-music-gtk4
# archlinuxcn repo
sudo pacman -Syu netease-cloud-music-gtk4
```

### Ubuntu(24.10/24.04/22.04)
```
# 添加 PPA 源
sudo add-apt-repository ppa:gmg137/ncm
# 刷新源
sudo apt update
# 安装
sudo apt install netease-cloud-music-gtk
```

### Debian
> [添加 Debian 中文社区软件源](https://github.com/debiancn/repo/blob/master/README.rst)
```
# 安装
sudo apt install netease-cloud-music-gtk
```

### Flatpak
#### 从 Flathub 安装
<a href='https://flathub.org/apps/com.github.gmg137.netease-cloud-music-gtk'>
    <img width='240' alt='Download on Flathub' src='https://flathub.org/api/badge?locale=zh-Hans'/>
</a>

#### 离线安装
```bash
sudo flatpak install com.gitee.gmg137.NeteaseCloudMusicGtk4-*.flatpak
```

### Nix
```bash
nix-env -iA nixpkgs.netease-cloud-music-gtk
```

### Gentoo Linux
```
// 添加gentoo-zh源
sudo emerge --ask app-eselect/eselect-repository
sudo eselect repository enable gentoo-zh
// 同步gentoo-zh源
sudo emerge --sync gentoo-zh
// 安装
sudo emerge --ask media-sound/netease-cloud-music-gtk
```

### 从源码安装(不推荐)
> Linux 编译依赖: openssl、dbus、gtk4、gdk-pixbuf、libadwaita-1、gstreamer、gstreamer-base（Windows 见下文 MSVC 说明，不使用 dbus 包）
```
// 安装依赖（Debian）
sudo apt-get install -y libssl-dev meson rustc libgtk-4-dev libadwaita-1-dev libgstreamer-plugins-bad1.0-dev

// 下载源码
git clone https://github.com/gmg137/netease-cloud-music-gtk.git
cd netease-cloud-music-gtk

// 编译
meson _build
cd _build
ninja

// 安装
sudo ninja install
```

Linux 与 Windows 均使用 Meson + Cargo 构建，但依赖引导和最终打包按目标平台分离。Windows MSVC 请参阅 [build-aux/windows/README.md](build-aux/windows/README.md)。

## FAQ
1. 为什么后台运行时没有托盘图标?
> Linux 版通过 ksni 提供系统托盘；Windows 首版暂不提供托盘，因此关闭窗口会直接退出。
2. Windows 上双击 exe 提示缺少 DLL？
> 请运行 `_windows\dist\netease-cloud-music-gtk4-<版本>-windows-x64\`（或对应 zip 解压目录）里的 exe，而不是 `_windows\install\bin\` 下的裸文件。
3. 使用 osdlyrics 时没有正确匹配歌词?
> 打开 osdlyrics 的[首选项]-[歌词位置]-[文件名]，添加匹配规则: %t-%p-%a。
4. 音乐缓存目录在什么位置?
> Linux 通常位于 `~/.cache/netease-cloud-music-gtk4`；Windows 位于 GLib 对应的用户 AppData 缓存目录。
5. 如何分享音乐?
> 点击播放栏的歌曲名称，便会复制歌曲链接等信息到剪贴板。
6. 如何查看日志
> 从终端启动程序，添加环境变量 RUST_LOG=debug 或 RUST_LOG=netease_cloud_music_gtk4。

## 截图
![](./screenshots/discover.png)
![](./screenshots/discover-dark.png)
![](./screenshots/toplist.png)


## License
This project's source code and documentation is licensed under the  [GNU General Public License](COPYING) (GPL v3).

## 参考
- [Shortwave](https://gitlab.gnome.org/World/Shortwave)
- [gnome-music](https://gitlab.gnome.org/GNOME/gnome-music)
