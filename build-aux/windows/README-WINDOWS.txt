网易云音乐 GTK4 - Windows x64 便携版

运行方式
--------
解压完整目录后，双击 netease-cloud-music-gtk4.exe。请勿只复制 exe；
GTK、GStreamer、GSettings schema、图标和界面资源都随目录分发。

系统要求
--------
- Windows 10 1809 或更高版本，64 位
- Microsoft Visual C++ 2015-2022 Redistributable (x64)

当前限制
--------
- 不提供 Linux MPRIS 媒体控制
- 不提供 ksni 系统托盘和“退出到后台”
- 不提供依赖 desktop-lyrics/osdlyrics 的外部桌面歌词
- 应用内歌词、播放、登录、搜索和缓存功能保持可用

诊断
----
可在 PowerShell 中运行：

  $env:RUST_LOG = "debug"
  $env:GST_DEBUG = "2"
  .\netease-cloud-music-gtk4.exe

用户数据、登录 Cookie 和缓存保存在 Windows 用户数据目录中。Cookie 属于
敏感信息，反馈问题时请勿上传 cookies.json。
