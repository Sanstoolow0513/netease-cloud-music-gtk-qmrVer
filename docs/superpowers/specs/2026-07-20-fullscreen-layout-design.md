# 三主页面全屏布局统一设计（限宽居中）

日期：2026-07-20
状态：已获用户确认，待写实现计划

## 背景与问题

「我的 / 发现 / 榜单」三个主页面在全屏（1920px+）宽窗口下布局失衡，用户确认的三个痛点：

1. **元素变形 / 空旷稀疏** —— 我的页快捷入口卡片（设计 140×140）被 GtkFlowBox 拉伸成宽扁矩形；发现页 Banner 固定高 320px 但宽度全满，图片被严重放大裁切；我的页推荐歌单区块上下大片空白。
2. **超宽难阅读** —— 榜单页 `SongListView` 的 `clamp-maximum-size` 此前被调为 100000（等于取消限宽），歌曲行全宽，歌名与时长相距近 2000px。
3. **缺乏视觉重心** —— 内容散满全屏，无聚拢感。

关键事实（已核实）：

- `GtkFlowBox` 会把子项拉伸到均分列宽，除非子项根容器 `halign != fill`——这是卡片变形的机理。
- 发现页外层已有 `ScrolledWindow + GtkViewport`（window.ui 中 `discover_view`）；**我的页没有外层滚动容器**，靠推荐歌单区块内部的 ScrolledWindow `vexpand` 撑高度，导致内容纵向被扯散（截图中标题位于页面中部的根因）。
- `SongListView` 组件自带 `AdwClamp`（默认 `maximum-size: 1000`），被歌单详情页等多处复用。

## 已确认的决策

| 决策点 | 结论 | 备选（未采用） |
|---|---|---|
| 布局方向 | **A. 单档限宽居中**（Apple Music 式） | B. 双档限宽（浏览宽/阅读窄）；C. 保持全宽只修变形 |
| 限宽栏宽度 | **1280px** | 1400 / 1600 |
| Banner 显示 | **B. 按图片比例自适应高度，零裁切**；落地为 980 居中 + GtkAspectFrame(2.5806) 比例锁定 | A. 固定 320 铺满（仍裁切）；C. 原图居中+模糊背景（成本高） |

用户明确表示不希望靠"宽屏多塞几列"来填满屏幕。

## 总体规范

三页统一引入 **1280px 限宽栏**，用 `AdwClamp` 实现（GTK CSS 无 max-width）：

- `maximum-size: 1280`；`tightening-threshold` 保持 AdwClamp 默认 400（即可用宽度超过 1280+400=1680 时才收紧：窗口 ≥1680 出现两侧留白，1280~1680 之间贴满——有意决策，经用户确认；小屏零影响）。
- Clamp 套在**滚动容器之内、页面内容之外**；Clamp 自身不带 margin，横向留白仍由现有 `.page-content`（24px）保证。
- 卡片类元素一律**固定尺寸、禁止随窗口拉伸**：FlowBox 子项根容器加 `halign: center` + 固定 `width-request`。
- `modern.css` 只新增布局类（如需要），继续遵守头注约束：只用 Libadwaita 命名色。

## 发现页

- **Banner**：去掉 GtkOverlay 写死的 height-request: 320；外套 AdwClamp(maximum-size=980, tightening-threshold=0) + GtkAspectFrame(ratio=2.5806，即 1200/465 的宽/高)。图片统一下载为 1200×465，比例锁定后任何窗口宽度下零裁切，宽度 ≤980 居中、高度 ≈ 宽度×465/1200（上限约 380）。（落地决策，经用户确认：宽度收至 ~980 居中、高度恒定，而非满栏宽自适应。）
- **热门推荐 / 新碟上架**：卡片固定 160px 封面（不被拉宽），限宽栏内每行自然排约 6 张。
- Clamp 加在现有 Viewport（`discover_view`）与根 Box 之间，结构改动小。

## 我的页

- **整页改为外层滚动**：骨架改为 `ScrolledWindow → Viewport → AdwClamp(1280) → 根 Box`，与发现页一致；删除推荐歌单区块内部的 ScrolledWindow。
- **快捷入口**：6 张卡片固定 140×140（根容器 `halign: center` 防拉伸），在 1280 栏内一行排下。
- **推荐歌单**：FlowBox 直接放在限宽栏内，高度由内容决定、顶端对齐；内容超高时整页滚动。消除标题悬空与底部空洞。

## 榜单页

- 右侧内容区（头部 + 歌曲列表）统一收进 1280 限宽栏；左侧榜单侧栏保持 220px 不变。
- 榜单页把 `SongListView` 的 `clamp-maximum-size` 从 100000 改为 **1280**（组件默认值 1000 不动，不影响复用方）。
- 头部（封面 + 标题 + 播放按钮）套同一 1280 Clamp，保证封面左缘与列表左缘对齐。
- 收益：歌名→时长距离从 ~1900px 回到 1280px。

## 影响文件

- `data/gtk/discover.ui` —— 套 Clamp；Banner 去固定高度
- `data/gtk/my-page.ui` —— 骨架重构（外层滚动 + Clamp）；快捷入口防拉伸；删内部 ScrolledWindow
- `data/gtk/window.ui` —— 我的页挂载处如在此处加滚动容器（实现时与 my-page.ui 二选一，以不重复嵌套滚动为准）
- `data/gtk/toplist.ui` —— 右侧头部套 Clamp；`clamp-maximum-size` 100000 → 1280
- `src/gui/discover.rs` —— Banner 高度按图片比例计算（含栏宽监听）
- `src/gui/my_page.rs` —— 配合骨架调整（如有内部滚动假设）
- `data/themes/modern.css` —— 如需新增布局辅助类
- 无新增 `.rs` 文件则不动 `src/meson.build`；若新增翻译字符串需登记 `po/POTFILES`

## 验证

- `make run` 手工验证三页在 **~1000 / 1440 / 1920+** 三档窗口宽度下：卡片不变形、Banner 不裁切、无异常空白、小窗贴满无留白异常
- `ninja -C _build test`（desktop/metainfo/gschema 数据校验）
- `cargo clippy`
- 仓库无 Rust 单元测试，遵循"编译通过 + 手工运行验证"惯例

## 明确不做（YAGNI）

- 不做双档限宽（浏览宽/阅读窄）
- 不做 Banner 模糊背景填充
- 不改 `SongListView` 组件默认 `maximum-size: 1000`（歌单详情页等复用方不受影响）
- 不调整左侧榜单侧栏宽度
- 卡片固定尺寸，每行数量随栏宽自然变化（既有行为保留）；不做"宽屏下加大卡片或主动填满屏幕"的填充式设计
