# 三主页面全屏 1280px 限宽居中改造 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让「我的 / 发现 / 榜单」三个主页面在全屏宽窗口下内容收进 1280px 居中栏，修复卡片变形、Banner 裁切、榜单超宽三大痛点。

**Architecture:** 三页统一用 `AdwClamp(maximum-size=1280)` 作为限宽层，套在滚动容器之内、页面内容之外；发现页 Banner 用 `AdwClamp(980)` + `GtkAspectFrame(ratio=0.3875)` 实现零裁切定比例显示；卡片类元素固定尺寸防拉伸。除发现页卡片尺寸外，几乎全部为 `.ui` 声明式改动。

**Tech Stack:** GTK4 + Libadwaita（Rust gtk4-rs，CompositeTemplate + .ui 模板）、Meson 构建。

**设计依据：** `docs/superpowers/specs/2026-07-20-fullscreen-layout-design.md`（含 Banner 落地形态补充决策：宽度收至 ~980 居中、高度恒定 260~380、零裁切）。

## Global Constraints

- **本仓库没有 Rust 单元测试**（CLAUDE.md 明确约定）：验收 = `ninja -C _build`（重建 gresource，校验 .ui XML）+ `cargo clippy` + `make run` 手工验证。每个任务的"测试"步骤即此组合，不要新增测试基建。
- `data/themes/modern.css` 头注约束：只允许 Libadwaita 命名色，禁止硬编码颜色值。本计划新增的 CSS 规则不含颜色。
- 不新增任何 `.rs` / `.ui` / `.css` 文件 → **不需要**动 `src/meson.build`、`data/netease_cloud_music_gtk4.gresource.xml`、`po/POTFILES`。
- 不新增用户可见字符串 → 不需要 gettext 处理。
- `window.ui` 中 `id="discover"`、`id="my_page"` 必须保留在原有对象上（`src/window.rs` 的 `TemplateChild` 依赖），嵌套层级可以变。
- 共享组件 `SongListGridItem`（`src/gui/songlist_grid_item.rs`）不改：搜索页等复用方保持现状；我的页推荐歌单卡片维持 140px。
- 提交信息沿用仓库中文风格（如 `feat: 发现页骨架统一——……`），末尾加 `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`。
- 当前分支 `feat/unified-main-pages`，直接在本分支提交。

**验证命令速查（各任务复用）：**

```bash
ninja -C _build          # 重建 gresource；.ui XML 不合法会在此报错
cargo clippy             # Rust 侧静态检查（.rs 有改动时）
ninja -C _build test     # desktop/metainfo/gschema 数据校验
make run                 # 启动应用手工验证（终端留意 Gtk-CRITICAL / template 报错）
```

---

### Task 1: 发现页——内容区套 1280 AdwClamp

**Files:**
- Modify: `data/gtk/window.ui`（discover ViewStackPage 内，约 111-117 行）

**Interfaces:**
- Consumes: 无（首个任务）
- Produces: 无接口变化。`Discover` 对象保留 `id="discover"`，TemplateChild 绑定不受影响。

- [ ] **Step 1: 修改 window.ui，在 Viewport 与 Discover 之间插入 AdwClamp**

找到 `data/gtk/window.ui` 中如下片段：

```xml
                                                                    <object class="GtkScrolledWindow">
                                                                        <child>
                                                                            <object class="GtkViewport" id="discover_view">
                                                                                <property name="scroll-to-focus">False</property>
                                                                                <child>
                                                                                    <object class="Discover" id="discover" />
                                                                                </child>
                                                                            </object>
                                                                        </child>
                                                                    </object>
```

将 `<object class="GtkViewport" id="discover_view">` 的 `<child>` 内容改为：

```xml
                                                                                <child>
                                                                                    <object class="AdwClamp">
                                                                                        <property name="maximum-size">1280</property>
                                                                                        <child>
                                                                                            <object class="Discover" id="discover" />
                                                                                        </child>
                                                                                    </object>
                                                                                </child>
```

（其余行不动；`AdwClamp` 不加 margin、不加 tightening-threshold——clamp 自身无 margin 时该阈值无作用，24px 内边距由 `Discover` 根上的 `.page-content` 提供。）

- [ ] **Step 2: 重建并运行验证**

```bash
ninja -C _build
make run
```

预期：构建无错误；切到「发现」页，窗口全屏（1920+）时 Banner 与卡片网格整体居中、两侧各约 320px 留白；窗口拖到 ~1000px 时内容贴满（仅 24px 内边距）。终端无 `Gtk-CRITICAL`、无 template 加载报错。

- [ ] **Step 3: 提交**

```bash
git add data/gtk/window.ui
git commit -m "feat: 发现页限宽——内容区套 1280px AdwClamp 居中

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: 发现页——Banner 限宽 980 + 比例锁定（零裁切）

**Files:**
- Modify: `data/gtk/discover.ui`（Banner 区，约 10-81 行）
- Modify: `data/themes/modern.css`（末尾追加一条规则）

**Interfaces:**
- Consumes: Task 1 的页面级 1280 Clamp（Banner 在栏内再限宽 980）
- Produces: 新 CSS 类 `.banner-aspect`（仅本任务使用）；`discover.rs` 的 `add_carousel()` **不改**——`Picture` 仍按 (1200, 465) 下载、`ContentFit::Cover`，因容器比例与图片比例一致（465/1200 = 0.3875），Cover 不产生裁切。

**原理：** Banner 图统一下载为 1200×465。`GtkAspectFrame(obey-child=false, ratio=0.3875)` 强制子项宽高比恒为 1200:465，外套 `AdwClamp(980)` 把宽度钳在 ≤980 → 高度恒为 宽度×0.3875（980 时恰为 380），任何窗口宽度下图片完整显示、零裁切。`GtkAspectFrame` 继承自 `GtkFrame`，默认会画边框，需用 `.banner-aspect` CSS 去掉。

- [ ] **Step 1: 修改 discover.ui 的 Banner 结构**

找到 `data/gtk/discover.ui` 中第一个 `<child>` 块（模板根 Box 的第一个子项，包含 `GtkOverlay` + `AdwCarouselIndicatorDots`），其开头为：

```xml
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <child>
                    <object class="GtkOverlay">
                        <property name="height-request">320</property>
```

改为（插入 `AdwClamp` + `GtkAspectFrame`，**删除 `height-request` 行**）：

```xml
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <child>
                    <object class="AdwClamp">
                        <property name="maximum-size">980</property>
                        <child>
                            <object class="GtkAspectFrame">
                                <property name="ratio">0.3875</property>
                                <property name="obey-child">False</property>
                                <style>
                                    <class name="banner-aspect" />
                                </style>
                                <child>
                                    <object class="GtkOverlay">
```

`GtkOverlay` 的闭合处原为：

```xml
                        </child>
                    </object>
                </child>
                <child>
                    <object class="AdwCarouselIndicatorDots">
```

改为（补两个闭合标签，IndicatorDots 保持在 Clamp 之外、原垂直 Box 内）：

```xml
                        </child>
                    </object>
                                </child>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="AdwCarouselIndicatorDots">
```

即最终结构：`Box(vertical) → [ AdwClamp(980) → GtkAspectFrame(0.3875) → GtkOverlay → (AdwCarousel + 两个切换按钮) ] + [ AdwCarouselIndicatorDots ]`。Overlay 内的 carousel、按钮等原有内容一行不动。

- [ ] **Step 2: modern.css 追加去边框规则**

在 `data/themes/modern.css` 的「===== 发现页 =====」小节内（`.banner-image` 规则之后）追加：

```css
/* Banner 比例锁容器：去掉 GtkAspectFrame 默认边框（布局规则，不涉及颜色） */
.banner-aspect,
.banner-aspect > border {
    border: none;
    box-shadow: none;
}
```

- [ ] **Step 3: 重建并运行验证**

```bash
ninja -C _build
make run
```

预期：「发现」页 Banner 宽度 ≤980 居中于内容栏，图片完整显示（对比 Task 1 之前：左右两侧构图内容不再被切）；Banner 外无多余边框；轮播左右切换按钮、底部圆点、点击跳转均正常；窗口从 ~800 拖到全屏，Banner 按比例缩放不变形。终端无 `Gtk-CRITICAL`。

- [ ] **Step 4: 提交**

```bash
git add data/gtk/discover.ui data/themes/modern.css
git commit -m "feat: 发现页 Banner——980 限宽居中 + 比例锁定，图片零裁切

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: 发现页——热门推荐/新碟上架卡片 140→160

**Files:**
- Modify: `src/gui/discover.rs:55`、`src/gui/discover.rs:71`

**Interfaces:**
- Consumes: Task 1 的 1280 栏（1280 − 48 内边距 = 1232，每张卡 160 + flowboxchild margin 24 = 184 → 每行 6 张）
- Produces: 无接口变化（`SongListGridItem::box_update_songlist` 第三个参数 `pic_size: i32` 仅传值变化）

- [ ] **Step 1: 修改两处 pic_size**

`src/gui/discover.rs` 第 55 行（`setup_top_picks` 内）：

```rust
        SongListGridItem::box_update_songlist(top_picks.clone(), &song_list, 140, false, &sender);
```

改为：

```rust
        SongListGridItem::box_update_songlist(top_picks.clone(), &song_list, 160, false, &sender);
```

第 71 行（`setup_new_albums` 内）：

```rust
        SongListGridItem::box_update_songlist(new_albums.clone(), &song_list, 140, true, &sender);
```

改为：

```rust
        SongListGridItem::box_update_songlist(new_albums.clone(), &song_list, 160, true, &sender);
```

- [ ] **Step 2: 静态检查并运行验证**

```bash
cargo clippy
make run
```

预期：clippy 无新增警告；「发现」页全屏下热门推荐/新碟上架每行 6 张卡片、封面方形 160px 不被拉伸。

- [ ] **Step 3: 提交**

```bash
git add src/gui/discover.rs
git commit -m "feat: 发现页卡片尺寸 140→160，限宽栏内每行 6 张

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: 我的页——外层滚动 + 1280 Clamp + 删除内部滚动区

**Files:**
- Modify: `data/gtk/window.ui`（my_login ViewStackPage 内，约 194-199 行）
- Modify: `data/gtk/my-page.ui`（推荐歌单区块，约 242-295 行）

**Interfaces:**
- Consumes: 无
- Produces: 无接口变化。`MyPage` 保留 `id="my_page"`；`rec_grid`（`TemplateChild<gtk::FlowBox>`，`src/gui/my_page.rs:67`）保留 id 且类型不变。`window.rs` 的 `switch_my_page_to_login()` 按 ViewStackPage 名 `my_login` 切换，不受影响。

**背景：** 我的页此前没有外层滚动容器，靠推荐歌单区块内部 ScrolledWindow `vexpand` 撑满高度，导致全屏下标题悬在页面中部、上下大片空白。改为与发现页一致的骨架：外层统一滚动，内容顶端对齐。

- [ ] **Step 1: window.ui 给我的页挂载处加滚动 + Clamp**

找到 `data/gtk/window.ui` 中：

```xml
                                                            <object class="AdwViewStackPage">
                                                                <property name="name">my_login</property>
                                                                <property name="child">
                                                                    <object class="MyPage" id="my_page" />
                                                                </property>
                                                            </object>
```

改为：

```xml
                                                            <object class="AdwViewStackPage">
                                                                <property name="name">my_login</property>
                                                                <property name="child">
                                                                    <object class="GtkScrolledWindow">
                                                                        <child>
                                                                            <object class="GtkViewport">
                                                                                <property name="scroll-to-focus">False</property>
                                                                                <child>
                                                                                    <object class="AdwClamp">
                                                                                        <property name="maximum-size">1280</property>
                                                                                        <child>
                                                                                            <object class="MyPage" id="my_page" />
                                                                                        </child>
                                                                                    </object>
                                                                                </child>
                                                                            </object>
                                                                        </child>
                                                                    </object>
                                                                </property>
                                                            </object>
```

- [ ] **Step 2: my-page.ui 删除推荐歌单的内部滚动容器**

找到 `data/gtk/my-page.ui` 中：

```xml
                <child>
                    <object class="GtkScrolledWindow">
                        <property name="vexpand">true</property>
                        <child>
                            <object class="GtkViewport">
                                <child>
                                    <object class="GtkFlowBox" id="rec_grid">
                                        <property name="hexpand">True</property>
                                        <property name="valign">start</property>
                                        <property name="max-children-per-line">12</property>
                                        <property name="min-children-per-line">3</property>
                                        <property name="homogeneous">False</property>
                                        <property name="selection-mode">none</property>
                                        <property name="activate-on-single-click">True</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                    </object>
                </child>
```

改为（FlowBox 原位保留，去掉外层包裹）：

```xml
                <child>
                    <object class="GtkFlowBox" id="rec_grid">
                        <property name="hexpand">True</property>
                        <property name="valign">start</property>
                        <property name="max-children-per-line">12</property>
                        <property name="min-children-per-line">3</property>
                        <property name="homogeneous">False</property>
                        <property name="selection-mode">none</property>
                        <property name="activate-on-single-click">True</property>
                    </object>
                </child>
```

- [ ] **Step 3: my-page.ui 去掉推荐歌单区块的 vexpand**

同一文件中，推荐歌单区块的根 Box 原为：

```xml
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <property name="hexpand">true</property>
                <property name="vexpand">true</property>
```

删除 `<property name="vexpand">true</property>` 一行（`hexpand` 保留）。

- [ ] **Step 4: 重建并运行验证**

```bash
ninja -C _build
make run
```

预期：登录状态下「我的」页全屏时——快捷入口与「推荐歌单」标题顶端依次排列（标题不再悬在页面中部）；推荐歌单卡片下方无大片空洞；缩小窗口高度时整页可滚动；内容超宽时限宽居中（两侧留白与发现页一致）。终端无 `Gtk-CRITICAL`、无 template 报错。

- [ ] **Step 5: 提交**

```bash
git add data/gtk/window.ui data/gtk/my-page.ui
git commit -m "feat: 我的页骨架——外层滚动 + 1280 限宽，删除内部滚动区

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: 我的页——快捷入口卡片防拉伸

**Files:**
- Modify: `data/gtk/my-page.ui`（快捷入口 6 张卡片，约 18-240 行）

**Interfaces:**
- Consumes: Task 4 的页面骨架
- Produces: 无接口变化

**原理：** `GtkFlowBox` 会把子项分配到均分列宽，子项根容器 `halign` 默认为 `fill` 于是被拉伸——快捷入口的封面 Box（`width-request: 140`）在全屏下被拉成 ~312px 宽的扁矩形。给每张卡片根 Box 加 `halign: center` 即恢复 140px 方形居中。

- [ ] **Step 1: 给 6 张快捷入口卡片根 Box 加 halign=center**

`data/gtk/my-page.ui` 中，6 张快捷入口卡片的根容器均为如下片段（共出现 6 次，仅图标/文案/回调不同）：

```xml
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
```

用**全部替换**（replace_all）改为：

```xml
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <property name="halign">center</property>
```

注意：该「vertical + spacing 8」两行组合在文件中恰好只出现于这 6 张卡片（根 Box 是 spacing 32，推荐歌单区块是 spacing 12；另有一处 horizontal + spacing 8 的分区标题栏不受影响），替换前先确认匹配数为 6：

```bash
grep -A1 'orientation">vertical' data/gtk/my-page.ui | grep -c 'spacing">8'   # 预期输出: 6
```

- [ ] **Step 2: 重建并运行验证**

```bash
ninja -C _build
make run
```

预期：「我的」页全屏时 6 张快捷入口卡片恢复 140×140 方形、在栏内均匀分布；点击各入口跳转正常。

- [ ] **Step 3: 提交**

```bash
git add data/gtk/my-page.ui
git commit -m "fix: 我的页快捷入口卡片防拉伸，恢复 140 方形

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: 榜单页——右侧内容区整体 1280 Clamp

**Files:**
- Modify: `data/gtk/toplist.ui`（约 29-117 行）

**Interfaces:**
- Consumes: `SongListView` 已有的 `clamp-maximum-size` / `clamp-margin-*` 属性（`src/gui/songlist_view.rs:240` 起，转发到内部 `adw_clamp`）
- Produces: 无接口变化；`SongListView` 组件默认值（1000）不动，其他复用方（歌单详情页等）不受影响

**对齐原理：** 把整个右侧垂直 Box 套进一个 `AdwClamp(1280)`。栏内：头部 `.page-header`（padding 24）内容宽 1232；`SongListView` 的 `clamp-margin-start/end` 24 + 内部 clamp（maximum 1280，远大于可用宽度而惰性）→ 列表内容同为 1232，左右缘自然对齐。`clamp-maximum-size` 从 100000 改回 1280 是声明意图、并防止未来布局变化时失控。

- [ ] **Step 1: toplist.ui 右侧 Box 外套 AdwClamp**

找到 `data/gtk/toplist.ui` 中 `GtkPaned` 的第二个 `<child>`：

```xml
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">12</property>
                        <child>
```

改为：

```xml
                <child>
                    <object class="AdwClamp">
                        <property name="maximum-size">1280</property>
                        <child>
                            <object class="GtkBox">
                                <property name="orientation">vertical</property>
                                <property name="spacing">12</property>
                                <child>
```

该垂直 Box 的闭合处（`</object>` 后紧跟 `</child>` 再 `</object>` 闭合 GtkPaned）原为文件末尾：

```xml
                        </child>
                    </object>
                </child>
            </object>
        </child>
    </template>
```

改为（补 Clamp 的闭合标签）：

```xml
                        </child>
                            </object>
                        </child>
                    </object>
                </child>
            </object>
        </child>
    </template>
```

即最终结构：`GtkPaned → [ 侧栏 ScrolledWindow ] + [ AdwClamp(1280) → GtkBox(vertical) → (头部 Box + SongListView) ]`。头部 Box 与 SongListView 内部一行不动。

- [ ] **Step 2: SongListView 的 clamp-maximum-size 改回 1280**

同一文件中：

```xml
                            <object class="SongListView" id="songs_list">
                                <property name="clamp-maximum-size">100000</property>
```

改为：

```xml
                            <object class="SongListView" id="songs_list">
                                <property name="clamp-maximum-size">1280</property>
```

（`clamp-margin-top/bottom/start/end` 各行保持不动。）

- [ ] **Step 3: 重建并运行验证**

```bash
ninja -C _build
make run
```

预期：「榜单」页全屏时——右侧头部与歌曲列表整体限宽居中，封面左缘与列表左缘对齐、「播放全部」按钮在栏内右缘；歌曲行歌名到时长距离明显收窄（≤1232px）；左侧榜单侧栏宽度不变、切换榜单正常；窗口拖到 ~1000px 时内容贴满。终端无 `Gtk-CRITICAL`。

- [ ] **Step 4: 提交**

```bash
git add data/gtk/toplist.ui
git commit -m "feat: 榜单页限宽——右侧内容区套 1280px AdwClamp，头部与列表对齐

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: 全量回归验证

**Files:**
- 无改动（仅验证）

**Interfaces:**
- Consumes: Task 1-6 全部改动
- Produces: 无

- [ ] **Step 1: 静态与数据校验**

```bash
cargo clippy
ninja -C _build test
```

预期：clippy 无新增警告；数据校验全部通过。

- [ ] **Step 2: make run 三档宽度手工回归**

```bash
make run
```

在窗口宽度约 1000px（或半屏平铺）、1440px、全屏 1920px+ 三档下，逐页检查：

发现页：
- [ ] Banner 图片完整（左右构图不被切）、宽度 ≤980 居中、外圈无多余边框
- [ ] 轮播切换按钮、圆点、点击跳转、自动轮播正常
- [ ] 热门推荐/新碟上架每行 6 张、封面方形

我的页（登录态）：
- [ ] 快捷入口 6 张 140 方形卡片，点击各入口正常
- [ ] 「推荐歌单」标题紧跟快捷入口（不再悬空中部），卡片下方无异常空白
- [ ] 窗口高度缩小时整页可滚动

榜单页：
- [ ] 头部封面左缘与歌曲列表左缘对齐
- [ ] 歌曲行不再横贯全屏；悬停行操作按钮、双击播放正常
- [ ] 左侧榜单切换正常，侧栏宽度不变

通用：
- [ ] 三页在 1920+ 下两侧留白一致、视觉重心居中；~1000px 下全部贴满无异常
- [ ] 终端全程无 `Gtk-CRITICAL` / template 加载报错
- [ ] 明暗两主题下各页无样式异常（可在偏好设置切换）

- [ ] **Step 3: 如发现回归，修复后在对应任务内补提交；全部通过后收工**

`git log --oneline` 应看到本计划 6 个功能提交（Task 1-6）。
