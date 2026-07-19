# 展示内容 UI 现代化重构设计

- 日期：2026-07-19
- 状态：已获用户确认（三节设计均已过目）
- 范围：全局展示内容 UI（歌曲列表行、歌单卡片、详情页头部、发现页分区），不含播放栏与播放逻辑

## 目标

将应用的展示内容 UI 统一升级为「现代化 Adwaita 原生风」：保持 GTK4/Libadwaita 原生质感，
参考新版 GNOME 应用的圆角、留白与卡片语言，引入强调色体系，明暗主题自动适配。

## 用户已确认的关键决策

| 决策点 | 结论 |
|---|---|
| 重构范围 | 全局整体改造（列表行 + 卡片 + 详情页头部 + 发现页分区） |
| 风格方向 | 现代化 Adwaita 原生风（非 Spotify/Apple Music/网易云官方风） |
| 列表行封面 | 纯文字精炼版，不加封面缩略图 |
| 卡片风格 | 封面大圆角 + 悬停浮现播放按钮 |
| 详情页头部 | 大封面 + 信息 + 主操作按钮 |
| 行交互 | 悬停浮现行尾按钮 + 播放态高亮；保留单击播放 |
| 卡片浮层点击行为 | 进入歌单详情页（复用现有 Action，不新增 API 链） |
| 详情页简介 | 展示 description，最多 2 行省略 |
| 实现策略 | CSS + 模板微调为主，仅歌单卡片控件重写；不动 ListBox 渲染架构、零新增 Action |

## 架构与实现策略

集中式样式 + 局部控件调整，分三层：

1. **新增 `data/themes/modern.css`**：统一承载全部现代化视觉规则（行高、圆角、悬停态、
   播放态高亮、强调色），避免样式散落在各 `.ui` 模板中。登记进
   `data/netease_cloud_music_gtk4.gresource.xml`，CSS provider 挂载跟随
   `discover.css` 现有模式（`STYLE_PROVIDER_PRIORITY_APPLICATION`，display 级注入）。
2. **`.ui` 模板微调**：`songlist-row.ui`、`songlist-page.ui`、`toplist.ui`、`discover.ui`
   仅调整样式类、尺寸与局部结构。
3. **代码局部改动**：`songlist_grid_item.rs`（卡片结构重写）、`songlist_row.rs`
  （playing 样式类切换、手势限制主键）、`songlist_page.rs`（简介绑定、封面尺寸）。

## 组件设计

### 1. 视觉基础（modern.css）

- 歌曲行高：去掉模板写死的 `height-request=59`，改由 CSS padding 控制到 ~44px，提高列表密度。
- 圆角：行/卡片容器 8-12px；封面 12px。
- 悬停态：行背景 `alpha(@theme_fg_color, 0.06)` 级别。
- 强调色体系：引入 `@accent_bg_color` —— 播放中歌曲名着色、主操作按钮 `suggested-action` 胶囊样式；
  由 Libadwaita 自动处理明暗主题，不硬编码颜色。
- 无版权置灰逻辑（`.song_row` opacity）从 discover.css 迁入 modern.css，行为不变。

### 2. 歌曲列表行（songlist-row.ui / songlist_row.rs）

- 三列 `GtkSizeGroup` 等分结构保留（对齐成本低）。
- 行容器加圆角与悬停背景；`boxed-list` 分隔保留但更轻。
- **行尾悬停按钮**：like / album / remove 三按钮默认 `opacity: 0`，行 `:hover` 时淡入
  （GTK CSS opacity 过渡）；不悬停时只显示时长文本。
- **播放态高亮**：`SonglistRow` 在 `switch_image` 处同步增删 `playing` 样式类；
  播放中行歌名用 `@accent_bg_color` 着色，原小播放图标保留作位置指示。
- 行点击手势补 `button=1` 限制，修正右键 release 也触发播放的问题。

### 3. 歌单卡片（songlist_grid_item.rs，纯代码构建）

- 结构改为 `GtkOverlay`：底层封面 `GtkPicture`（保持宽高比裁剪）+ 右下角圆形播放浮层按钮。
- 封面圆角 12px + 细微 `box-shadow`，替代裸 `GtkFrame`；占位 `image-missing-symbolic` 同样受圆角容器约束。
- 悬停时封面轻微提亮、浮层按钮淡入；点击浮层按钮进入歌单详情页（复用现有 Action，零新增 API 调用）。
- 标题保持 2 行省略；作者行 `dim-label` + `caption` 字号。
- FlowBox 路径（发现页）与 GridView 路径（搜索歌单页 `setup_factory`）共用同一构建函数，两处同步生效。

### 4. 详情页头部（songlist-page.ui / toplist.ui）

- 封面 140px → ~200px，`GtkFrame` → 圆角 `GtkPicture` + 阴影，与卡片统一。
- 信息区：标题 `title-1`（去掉 27 字符截断，改 2 行 wrap）→ 元信息行
  （歌曲数 · 收藏数/发布时间，`dim-label`）→ 简介行（绑定已有的 description 数据，2 行 ellipsize，为空时隐藏）。
- 按钮组：主按钮改为「播放全部」胶囊按钮（`suggested-action` + pill + 播放图标 + 文字），
  收藏按钮保留为 circular 次按钮。
- 榜单页右侧头部复用同一结构，顺带统一。

### 5. 发现页微调（discover.ui，不动结构）

- 轮播 banner 圆角加大、指示点样式微调。
- 分区标题从手写 `size=15000` 换标准 `title-3` 样式类；「View More」按钮样式统一。
- FlowBox 卡片间距纳入 modern.css 管理。

## 数据流

无变化。仍走既有「GUI 发 Action → Application 处理 → 回发 Action 更新 UI」模式；
本次重构**零新增 Action**，图片加载仍用 `set_from_net` / 文件缓存机制（`model.rs` ImageDownloadImpl）。

## 错误处理与降级

- 封面加载失败/未加载：保持 `image-missing-symbolic` 占位，圆角容器对占位同样生效。
- 简介为空：简介行隐藏，不占垂直空间。
- 所有颜色只引用 Libadwaita 命名色（`@accent_bg_color`、`@theme_fg_color` 等），明暗主题自动适配。
- opacity 过渡在低性能环境自然降级为直接显隐。

## 验证计划

项目无 Rust 单元测试，按 AGENTS.md 惯例：

1. `cargo build` 与 meson 全量构建（`_build` 目录 `ninja`）零警告。
2. `meson test`（desktop/metainfo/gschema 校验）确认不受影响。
3. 手工验证清单（明/暗主题各一遍）：发现页（卡片悬停、轮播）、榜单页（切榜、行悬停、
   播放态高亮）、歌单详情页（简介、播放全部）、搜索三页、我的页。
4. `RUST_LOG=debug` 运行确认无新增 GTK critical 警告。

## 影响文件清单

- 新增：`data/themes/modern.css`（+ gresource 登记）
- 模板：`data/gtk/songlist-row.ui`、`songlist-page.ui`、`toplist.ui`、`discover.ui`
- 代码：`src/gui/songlist_grid_item.rs`、`src/gui/songlist_row.rs`、`src/gui/songlist_page.rs`、
  CSS provider 挂载点 `src/window.rs`（modern.css 含全局行/卡片样式，需 display 级全局挂载，
  不依赖 Discover 页是否已实例化）
- 不动：ListBox 渲染架构、Action 总线、播放逻辑、GSettings、图片缓存机制
