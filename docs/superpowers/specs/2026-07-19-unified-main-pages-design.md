# 三主页面设计统一与全屏适配设计

- 日期：2026-07-19
- 状态：已获用户确认（三节设计均已过目）
- 范围：发现 / 榜单 / 我的 三个主标签页的布局骨架、间距、分区标题、卡片网格与全屏行为；不含歌单详情页、搜索页、播放栏与播放逻辑
- 前置工作：本文档建立在 `2026-07-19-modern-content-ui-design.md`（已合并）之上，沿用「现代化 Adwaita 原生风 + 集中式 modern.css」路线

## 目标

解决三主页面「内容不规范、全屏显示效果不好、三页效果不统一」的问题：建立一套统一设计规范（页面骨架 / 间距刻度 / 分区标题 / 卡片网格），三个页面全部对齐到该规范；宽窗口下内容流式填满（Spotify 式），而非限宽居中。

## 用户已确认的关键决策

| 决策点 | 结论 |
|---|---|
| 统一范围 | 仅发现 / 榜单 / 我的三主页面 |
| 全屏策略 | 流式填满（Spotify 风）：卡片网格列数随宽度增长，不做限宽居中 |
| 榜单页布局 | 保留左侧栏（GtkPaned 榜单列表），只统一间距与排版细节 |
| 发现页 Banner | 去写死 730×283，宽度自适应拉伸、保持宽高比、最大高度约 320px |
| 我的页快捷入口 | 6 个 100px 大头像行废弃，改为与歌单卡片同规范的标准卡片网格 |
| 实现路线 | 方案 A：规范先行 + modern.css 集中承载 + 三页模板对齐，Rust 改动最小，零新增 Action |

## 统一设计规范（modern.css 承载）

### 页面骨架与间距刻度

- 三页内容区统一：左右内边距 24px、顶部 16px、底部 24px；分区间距 32px；分区标题与内容间距 12px。
- 间距只用 4 / 8 / 12 / 16 / 24 / 32 六档；清理模板里的散数（9 / 13 / 15 / 18 / 20 / 58 等手写 margin、spacing）。
- 滚动架构尊重现状（发现页整页滚动、榜单页侧栏+列表滚动、我的页整页滚动），统一的是边距 / 排版 / 组件语言，不强行统一滚动容器结构。

### 颜色与字体

- 只用 Libadwaita 命名色（`@accent_bg_color`、`@theme_fg_color` 等），明暗主题自动适配；清掉 `discover.css` 里仅存的硬编码 `rgba(255,255,255,0.5)`。
- 分区标题统一模式：`图标 + title-3 文本 +（可选）右侧 flat「查看更多」按钮`。
- 我的页手写的 `size=15000` Pango 属性全部换成 `title-3` 样式类；辅助文本统一 `dim-label` / `caption`。

### 卡片网格统一（流式填满的核心机制）

- 卡片内容宽固定（封面 200px 基准），FlowBox 去掉 `max-children-per-line=4` 上限、保留 `min-children-per-line=3`：列数随窗口宽度自然增长，宽屏自动多列，零布局代码。
- 卡片行列间距统一收进 modern.css；清掉 `discover.css` 里 `.songlist_grid_page gridview` 的散距规则（该文件若清空则连同 gresource 登记一并移除）。

## 三个页面的具体改造

### 发现页（discover.ui / discover.rs）

- Banner 去写死尺寸：删掉 `discover.ui` 的 `height-request=283`、`discover.rs` 的 `set_width_request(730)` 与取图参数 `(730, 283)`；改为宽度随内容区拉伸、按原图宽高比自适应，最大高度约 320px；取图宽度调大到约 1200 保证全屏清晰。左右切换按钮与指示点保留。
- 两个分区（Top Picks / New Albums）套用统一标题模式与网格规范。
- 根容器从 `halign=center` 无约束状态改为 fill + 统一 24px 页面边距。

### 榜单页（toplist.ui）

- 侧栏保留：220px `navigation-sidebar`，只统一内边距与分隔间距。
- 内容区头部：去掉 `AdwClamp(1000)`，改为统一 24px 页面边距的流式头部（200px 封面 + 标题/元信息 + 「播放全部」胶囊按钮，现有结构不变）。
- 歌曲列表流式：`SongListView` 已暴露 `clamp-maximum-size` GObject 属性（`src/gui/songlist_view.rs`），直接在 `toplist.ui` 模板里调大该属性即可填满宽度——不动共享组件内部，歌单详情页 / 搜索页不受影响。

### 我的页（my-page.ui / my_page.rs）

- 快捷入口重做：6 个 100px `AdwAvatar` 行废弃，改为与歌单卡片同一视觉规范的卡片网格（圆角 12px 封面容器 + 阴影 + 悬停态），封面位放大号 symbolic 图标，标题置于图下；FlowBox 流式排列。点击处理复用现有 handler（`daily_rec_cb` / `heartbeat_cb` / `radio_cb` / `cloud_disk_cb` / `collection_album_cb` / `collection_songlist_cb`），零新增 Action。
- 「推荐歌单」分区：标题换统一 `title-3` 模式，网格去列上限，手写 Pango 属性全部清除。
- 页面边距与发现页一致；模板里 `spacing=58`、`size=15000` 等散数全部清理。

## 数据流

无变化。仍走「GUI 发 Action → Application 处理 → 回发 Action 更新 UI」模式，本轮零新增 Action；图片加载仍用现有 `set_from_net` / 文件缓存机制。

## 错误处理与降级

- Banner 加载失败 / 未加载：保持 `image-missing-symbolic` 占位 + 命名色占位背景，容器比例不塌陷。
- 快捷入口卡片图标全部用 symbolic 图标，明暗主题自动适配；新增样式只引用命名色，不新增硬编码颜色。
- 窄窗口（最小宽 750px）：卡片网格靠 `min-children-per-line=3` 与 FlowBox 自然换行降级；不引入响应式断点逻辑。

## 验证计划

项目无 Rust 单测，按 AGENTS.md 惯例：

1. `cargo build` 与 `_build` 下 `ninja` 全量构建零警告；`meson test`（desktop / metainfo / gschema 校验）不受影响。
2. 人工验证矩阵：明 / 暗主题 × 窄(750px) / 默认(1160px) / 全屏 三档窗口，过发现 / 榜单 / 我的三页（卡片列数随宽度变化、Banner 比例、悬停态、分区标题一致性）。
3. `RUST_LOG=debug` 运行确认无新增 GTK critical 警告。

## 影响文件清单

- 样式：`data/themes/modern.css`（规范规则）、`data/themes/discover.css`（清理或移除 + gresource 登记同步）
- 模板：`data/gtk/discover.ui`、`toplist.ui`、`my-page.ui`
- 代码：`src/gui/discover.rs`（Banner 尺寸逻辑）、`src/gui/my_page.rs`（快捷入口卡片网格构建）
- 不动：Action 总线、播放逻辑、`SongListView` 等共享组件内部、GSettings、图片缓存机制
