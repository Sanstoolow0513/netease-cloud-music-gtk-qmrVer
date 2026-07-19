# 三主页面设计统一与全屏适配实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立统一设计规范（页面骨架/间距刻度/分区标题/卡片网格），发现/榜单/我的三主页面全部对齐，宽窗口下内容流式填满。

**Architecture:** 规范规则集中追加到 `data/themes/modern.css`（display 级 provider 已在 `window.rs` 挂载，无需再动）；`discover.ui`、`toplist.ui`、`my-page.ui` 三模板按统一骨架重写；Rust 侧仅改 `discover.rs`（Banner 尺寸）、`my_page.rs`（去掉头像模板子控件）、`songlist_view.rs`（新增两个 clamp 边距透传属性）、删除 `data/themes/discover.css`。零新增 Action，零新增翻译字符串。

**Tech Stack:** Rust (gtk-rs 0.11 / libadwaita 0.9 v1_6)、GTK4 Builder 模板、GTK CSS、Meson + Cargo。

**设计规格：** `docs/superpowers/specs/2026-07-19-unified-main-pages-design.md`

## Global Constraints

- 项目无 Rust 测试框架（无 `#[test]`、无 dev-dependencies）。每个任务的"验证"= `cargo build` 零警告 + `cd _build && ninja` 通过（gresource 编译能抓出 ui/css 语法错误）。**不要**新增测试依赖。
- 所有颜色只允许引用 Libadwaita 命名色（`@accent_bg_color`、`@accent_fg_color`、`@theme_fg_color` 等），禁止硬编码颜色值，保证明暗主题自适应。
- 间距只使用 4 / 8 / 12 / 16 / 24 / 32 六档；清理模板中 9 / 13 / 15 / 18 / 20 / 58 等散数。
- 不新增任何 `Action` 枚举成员；不改动 `src/application.rs`。
- 本轮不新增用户可见字符串（六个快捷入口标签与分区标题复用现有 `translatable` 字符串），无需动 `po/`。
- 源文件头部保留既有版权注释块风格；Rust 4 空格缩进，改完跑 `cargo fmt`。
- 构建验证命令：`cargo build`（需 `_build` 已 meson setup 过）；gresource 验证：`cd _build && ninja`。
- 不要提交 `_build/`、`target/`、`src/config.rs`（均已被 gitignore）。

## 与规格的两处细化（执行时以此为准）

1. **Banner 高度采用「固定 320px + cover 裁剪」**：GTK CSS 无 `max-height` 机制，`GtkPicture` 的 height-for-width 在超宽屏会得出过高高度。务实做法：`GtkOverlay` 设 `height-request=320`，`GtkPicture` 设 `content-fit=cover` + `hexpand`，宽度任意拉伸、高度恒定 320、超出部分裁剪。符合规格「最大高度约 320px」的意图。
2. **`SongListView` 新增 `clamp-margin-start` / `clamp-margin-end` 两个透传属性**：规格说"不动共享组件内部"，但现有四个 `clamp-*` 属性不含左右边距，榜单页无法把列表左右边距统一到 24。按现有属性完全同构地新增两个（纯增量、默认值不变），歌单详情页/搜索页行为不受影响。

---

### Task 1: modern.css 规范层 + discover.css 清理移除

**Files:**
- Modify: `data/themes/modern.css`
- Delete: `data/themes/discover.css`
- Modify: `data/netease_cloud_music_gtk4.gresource.xml:21`
- Modify: `src/gui/discover.rs`（`class_init` 约 :151-155、文件末尾 `load_css` :286-297）
- Modify: `AGENTS.md`（modern.css 描述行）

**Interfaces:**
- Produces: CSS 类 `.page-content`（页面内容区统一内边距）、`.page-header`（榜单页头部内边距）、`.quick-entry-cover`（我的页快捷入口封面底色），供 Task 2/3/4 的模板使用。

- [ ] **Step 1: modern.css 追加规范规则**

更新 `data/themes/modern.css` 头部注释的职责描述行：

```css
 * 展示内容 UI 现代化样式：页面骨架、歌曲行、歌单卡片、详情页头部、发现页。
```

在文件末尾追加：

```css

/* ===== 页面骨架（三主页面统一规范） ===== */

/* 页面内容区统一内边距：左右 24、上 16、下 24 */
.page-content {
    padding-left: 24px;
    padding-right: 24px;
    padding-top: 16px;
    padding-bottom: 24px;
}

/* 榜单页头部（无滚动容器包裹，单独应用页面内边距） */
.page-header {
    padding-left: 24px;
    padding-right: 24px;
    padding-top: 16px;
}

/* ===== 我的页快捷入口卡片 ===== */

/* 封面位：强调色淡底 + 强调色图标，明暗主题自适应 */
.quick-entry-cover {
    background-color: alpha(@accent_bg_color, 0.12);
    color: @accent_bg_color;
}

/* ===== 从 discover.css 迁入的规则（原文件已删除） ===== */

/* 轮播图左右切换按钮悬停态（白色半透明，位于图片之上与主题无关） */
button.hover-button:hover {
    background-color: alpha(white, 0.5);
}

.label-album-grid-artist {
    font-size: smaller;
    color: @theme_fg_color;
}

/* 歌单网格页（搜索歌单页）间距，值已归一到间距刻度 */
.songlist_grid_page gridview {
    padding: 24px;
}

.songlist_grid_page gridview > child {
    margin: 16px;
    padding: 8px;
}
```

- [ ] **Step 2: 删除 discover.css 及其 gresource 登记**

```bash
rm data/themes/discover.css
```

`data/netease_cloud_music_gtk4.gresource.xml` 删除该行（:21）：

```xml
        <file compressed="true">themes/discover.css</file>
```

- [ ] **Step 3: discover.rs 移除 load_css**

`src/gui/discover.rs` 的 `class_init`（约 :151-155）改为：

```rust
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }
```

删除文件末尾整个函数（:286-297）：

```rust
fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_resource("/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/discover.css");

    // Add the provider to the default screen
    style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
```

（`discover.rs` 使用 `gtk::*` 通配导入，删除后无悬空 import。）

- [ ] **Step 4: AGENTS.md 同步 modern.css 描述**

`AGENTS.md` 代码结构一节中 `data/` 的说明行：

```
├── themes/*.css             # 自定义样式；modern.css 为集中式现代化展示样式（歌曲行/卡片/详情页头部/发现页），由 window.rs 在启动时按资源路径加载
```

改为：

```
├── themes/*.css             # 自定义样式；modern.css 为集中式现代化样式（页面骨架/歌曲行/卡片/详情页头部/发现页），由 window.rs 在启动时按资源路径加载
```

- [ ] **Step 5: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 均成功，无 warning；ninja 重编 gresource 不报错（若报 discover.css 找不到，说明 gresource 登记未删干净）。

- [ ] **Step 6: Commit**

```bash
git add data/themes/modern.css data/themes/discover.css data/netease_cloud_music_gtk4.gresource.xml src/gui/discover.rs AGENTS.md
git commit -m "refactor: 统一样式规范入 modern.css，清空并移除 discover.css"
```

---

### Task 2: 发现页骨架统一 + Banner 流式

**Files:**
- Modify: `data/gtk/discover.ui`（整体重写）
- Modify: `src/gui/discover.rs`（`add_carousel`，:83-111）

**Interfaces:**
- Consumes: Task 1 的 `.page-content` CSS 类。
- Produces: 模板子控件 id 不变（`carousel` / `previous_button` / `next_button` / `top_picks` / `new_albums`），`discover.rs` 的 `imp` 无需改动。

- [ ] **Step 1: 重写 discover.ui**

完整替换 `data/gtk/discover.ui` 为：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<interface>
    <requires lib="gtk" version="4.0" />
    <template class="Discover" parent="GtkBox">
        <property name="orientation">vertical</property>
        <property name="spacing">32</property>
        <style>
            <class name="page-content" />
        </style>
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <child>
                    <object class="GtkOverlay">
                        <property name="height-request">320</property>
                        <child>
                            <object class="AdwCarousel" id="carousel">
                                <property name="allow-scroll-wheel">false</property>
                                <signal name="notify::position" handler="carousel_notify_position_cb" swapped="true" />
                                <child>
                                    <object class="GtkGestureClick">
                                        <signal name="pressed" handler="carousel_pressed_cb" swapped="true" />
                                    </object>
                                </child>
                                <style>
                                    <class name="card" />
                                </style>
                            </object>
                        </child>
                        <child type="overlay">
                            <object class="GtkButton" id="previous_button">
                                <property name="can_focus">False</property>
                                <property name="halign">start</property>
                                <property name="valign">center</property>
                                <property name="width-request">39</property>
                                <property name="height-request">39</property>
                                <property name="margin-top">12</property>
                                <property name="margin-bottom">12</property>
                                <property name="margin-start">12</property>
                                <property name="margin-end">12</property>
                                <property name="icon-name">go-previous-symbolic</property>
                                <signal name="clicked" handler="previous_button_clicked_cb" swapped="true" />
                                <style>
                                    <class name="circular" />
                                    <class name="flat" />
                                    <class name="image-button" />
                                    <class name="hover-button" />
                                </style>
                            </object>
                        </child>
                        <child type="overlay">
                            <object class="GtkButton" id="next_button">
                                <property name="can_focus">False</property>
                                <property name="halign">end</property>
                                <property name="valign">center</property>
                                <property name="width-request">39</property>
                                <property name="height-request">39</property>
                                <property name="margin-top">12</property>
                                <property name="margin-bottom">12</property>
                                <property name="margin-start">12</property>
                                <property name="margin-end">12</property>
                                <property name="icon-name">go-next-symbolic</property>
                                <signal name="clicked" handler="next_button_clicked_cb" swapped="true" />
                                <style>
                                    <class name="circular" />
                                    <class name="flat" />
                                    <class name="image-button" />
                                    <class name="hover-button" />
                                </style>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="AdwCarouselIndicatorDots">
                        <property name="carousel">carousel</property>
                    </object>
                </child>
            </object>
        </child>
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <property name="hexpand">true</property>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">horizontal</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">media-optical-cd-audio-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="halign">start</property>
                                <property name="label" translatable="yes">Top Picks</property>
                                <style>
                                    <class name="title-3" />
                                </style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton">
                                <property name="halign">end</property>
                                <property name="hexpand">true</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="top_picks_cb" swapped="true" />
                                <style>
                                    <class name="flat" />
                                    <class name="image-button" />
                                </style>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkSeparator">
                        <property name="sensitive">False</property>
                        <property name="can_focus">False</property>
                    </object>
                </child>
                <child>
                    <object class="GtkFlowBox" id="top_picks">
                        <property name="hexpand">True</property>
                        <property name="max-children-per-line">12</property>
                        <property name="min-children-per-line">3</property>
                        <property name="homogeneous">False</property>
                        <property name="selection-mode">none</property>
                        <property name="activate-on-single-click">True</property>
                    </object>
                </child>
            </object>
        </child>
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <property name="hexpand">true</property>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">horizontal</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">media-optical-cd-audio-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="halign">start</property>
                                <property name="label" translatable="yes">New Albums</property>
                                <style>
                                    <class name="title-3" />
                                </style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton">
                                <property name="halign">end</property>
                                <property name="hexpand">true</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="new_albums_cb" swapped="true" />
                                <style>
                                    <class name="flat" />
                                    <class name="image-button" />
                                </style>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkSeparator">
                        <property name="sensitive">False</property>
                        <property name="can_focus">False</property>
                    </object>
                </child>
                <child>
                    <object class="GtkFlowBox" id="new_albums">
                        <property name="hexpand">True</property>
                        <property name="max-children-per-line">12</property>
                        <property name="min-children-per-line">3</property>
                        <property name="homogeneous">False</property>
                        <property name="selection-mode">none</property>
                        <property name="activate-on-single-click">True</property>
                    </object>
                </child>
            </object>
        </child>
    </template>
</interface>
```

要点：根容器去 `halign=center`、加 `.page-content`；Overlay 高 283→320、去 `margin-top=13`；分区标题间距归一（`spacing=8`，去 `margin-start=9`）；FlowBox `max-children-per-line` 4→12（列数随宽度增长）。

- [ ] **Step 2: discover.rs 的 add_carousel 去写死尺寸**

`src/gui/discover.rs` 的 `add_carousel`（:83-111）改为：

```rust
    pub fn add_carousel(&self, banner: BannersInfo) {
        let carousel = self.imp().carousel.get();

        if carousel.n_pages() == 2 {
            let widget = carousel.nth_page(1);
            carousel.scroll_to(&widget, false);
        }

        let mut path = CACHE.clone();
        path.push(format!("{}-banner.jpg", banner.target_id));

        let sender = self.imp().sender.get().unwrap().clone();
        let image = Picture::new();
        image.set_from_net(banner.pic.to_owned(), path, (1200, 465), &sender);

        image.set_halign(gtk::Align::Fill);
        image.set_valign(gtk::Align::Fill);
        image.set_hexpand(true);
        image.set_can_shrink(true);
        image.set_content_fit(gtk::ContentFit::Cover);
        image.add_css_class("banner-image");
        carousel.append(&image);
        self.imp().banners.borrow_mut().push(banner);
    }
```

要点：取图尺寸 (730,283)→(1200,465)（同宽高比 2.58，全屏不糊）；去 `set_width_request(730)`；`content-fit=cover` 保证任意宽度下填满 320px 高容器。

- [ ] **Step 3: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 均成功，无 warning。若报模板信号/子控件绑定错误，核对 id 与 handler 名是否与 `discover.rs` 完全一致。

- [ ] **Step 4: Commit**

```bash
git add data/gtk/discover.ui src/gui/discover.rs
git commit -m "feat: 发现页骨架统一——页面边距规范化、Banner 宽度自适应流式"
```

---

### Task 3: 榜单页统一（含 SongListView 边距属性扩展）

**Files:**
- Modify: `src/gui/songlist_view.rs`（`properties()` :236-238、`set_property()` :259-276、`property()` :284-287）
- Modify: `data/gtk/toplist.ui`（整体重写）

**Interfaces:**
- Produces: `SongListView` 新增 GObject 属性 `clamp-margin-start` / `clamp-margin-end`（int，默认 0，透传内部 `AdwClamp`），供 `toplist.ui` 及其他页面在模板中设置。
- 模板子控件 id 不变（`sidebar` / `cover_image` / `title_label` / `num_label` / `play_button` / `songs_list`），`toplist.rs` 无需改动。

- [ ] **Step 1: SongListView 新增两个边距属性**

`src/gui/songlist_view.rs` 的 `properties()` 中，在 `ParamSpecInt::builder("clamp-margin-bottom").build(),` 行后追加两行：

```rust
                    ParamSpecInt::builder("clamp-margin-start").build(),
                    ParamSpecInt::builder("clamp-margin-end").build(),
```

`set_property()` 中，在 `"clamp-margin-bottom" => { ... }` 分支后追加两个分支：

```rust
                "clamp-margin-start" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_start(val);
                }
                "clamp-margin-end" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_end(val);
                }
```

`property()` 中，在 `"clamp-margin-bottom" => ...` 分支后追加两个分支：

```rust
                "clamp-margin-start" => self.adw_clamp.margin_start().to_value(),
                "clamp-margin-end" => self.adw_clamp.margin_end().to_value(),
```

- [ ] **Step 2: 构建验证（属性注册正确性）**

```bash
cargo build 2>&1 | tail -5
```

Expected: 成功无 warning。

- [ ] **Step 3: 重写 toplist.ui**

完整替换 `data/gtk/toplist.ui` 为：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<interface>
    <template class="TopListView" parent="AdwBin">
        <child>
            <object class="GtkPaned">
                <property name="shrink-start-child">0</property>
                <child>
                    <object class="GtkScrolledWindow">
                        <property name="width-request">220</property>
                        <property name="margin-start">12</property>
                        <property name="margin-top">16</property>
                        <property name="margin-bottom">12</property>
                        <child>
                            <object class="GtkViewport">
                                <property name="scroll-to-focus">True</property>
                                <child>
                                    <object class="GtkListBox" id="sidebar">
                                        <property name="selection-mode">single</property>
                                        <signal name="row-activated" handler="sidebar_cb" swapped="yes" />
                                        <style>
                                            <class name="navigation-sidebar" />
                                        </style>
                                    </object>
                                </child>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">12</property>
                        <child>
                            <object class="GtkBox">
                                <property name="orientation">horizontal</property>
                                <property name="spacing">16</property>
                                <property name="hexpand">true</property>
                                <style>
                                    <class name="page-header" />
                                </style>
                                <child>
                                    <object class="GtkFrame">
                                        <style>
                                            <class name="songlist-cover-frame" />
                                        </style>
                                        <child>
                                            <object class="GtkImage" id="cover_image">
                                                <property name="pixel-size">200</property>
                                                <property name="icon-name">image-missing-symbolic</property>
                                            </object>
                                        </child>
                                    </object>
                                </child>
                                <child>
                                    <object class="GtkBox">
                                        <property name="orientation">vertical</property>
                                        <property name="halign">fill</property>
                                        <property name="valign">end</property>
                                        <property name="hexpand">true</property>
                                        <property name="spacing">8</property>
                                        <child>
                                            <object class="GtkLabel" id="title_label">
                                                <property name="label">Title</property>
                                                <property name="halign">start</property>
                                                <property name="wrap">True</property>
                                                <property name="lines">2</property>
                                                <property name="ellipsize">end</property>
                                                <style>
                                                    <class name="title-1" />
                                                </style>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkBox">
                                                <property name="orientation">horizontal</property>
                                                <property name="spacing">8</property>
                                                <child>
                                                    <object class="GtkLabel" id="num_label">
                                                        <property name="label">0 songs</property>
                                                        <property name="halign">start</property>
                                                    </object>
                                                </child>
                                                <child>
                                                    <object class="GtkButton" id="play_button">
                                                        <property name="halign">end</property>
                                                        <property name="hexpand">true</property>
                                                        <property name="tooltip-text" translatable="yes">Play songs list</property>
                                                        <signal name="clicked" handler="play_button_clicked_cb" swapped="true" />
                                                        <child>
                                                            <object class="AdwButtonContent">
                                                                <property name="icon-name">media-playback-start-symbolic</property>
                                                                <property name="label" translatable="yes">Play All</property>
                                                            </object>
                                                        </child>
                                                        <style>
                                                            <class name="suggested-action" />
                                                            <class name="pill" />
                                                        </style>
                                                    </object>
                                                </child>
                                            </object>
                                        </child>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="SongListView" id="songs_list">
                                <property name="clamp-maximum-size">100000</property>
                                <property name="clamp-margin-top">12</property>
                                <property name="clamp-margin-bottom">12</property>
                                <property name="clamp-margin-start">24</property>
                                <property name="clamp-margin-end">24</property>
                            </object>
                        </child>
                    </object>
                </child>
            </object>
        </child>
    </template>
</interface>
```

要点：头部 `AdwClamp(1000)` 去掉，改 `.page-header`（Task 1 定义）流式铺满；侧栏加 12/16 外边距；`songs_list` 通过 Step 1 新增的属性把左右边距统一为 24、`clamp-maximum-size` 放大实现列表流式。

- [ ] **Step 4: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 均成功，无 warning。若运行时报 `clamp-margin-start` 属性不存在，说明 Step 1 属性未正确注册。

- [ ] **Step 5: Commit**

```bash
git add src/gui/songlist_view.rs data/gtk/toplist.ui
git commit -m "feat: 榜单页统一——头部去限宽流式化，SongListView 补左右边距属性"
```

---

### Task 4: 我的页统一（快捷入口卡片网格）

**Files:**
- Modify: `data/gtk/my-page.ui`（整体重写）
- Modify: `src/gui/my_page.rs`（`imp::MyPage` 字段 :68-69、`daily_rec_cb` :92-104、`constructed` :137-149）

**Interfaces:**
- Consumes: Task 1 的 `.page-content` / `.quick-entry-cover` CSS 类；`.songlist-card-cover`（modern.css 已有）。
- Produces: 模板子控件 id 仅剩 `rec_grid`（`daily_rec_avatar` 移除）；六个 handler 名不变（`daily_rec_cb` / `heartbeat_cb` / `radio_cb` / `cloud_disk_cb` / `collection_album_cb` / `collection_songlist_cb`）。

- [ ] **Step 1: 重写 my-page.ui**

完整替换 `data/gtk/my-page.ui` 为：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<interface>
    <requires lib="gtk" version="4.0" />
    <template class="MyPage" parent="GtkBox">
        <property name="orientation">vertical</property>
        <property name="spacing">32</property>
        <style>
            <class name="page-content" />
        </style>
        <child>
            <object class="GtkFlowBox">
                <property name="hexpand">True</property>
                <property name="valign">start</property>
                <property name="max-children-per-line">12</property>
                <property name="min-children-per-line">3</property>
                <property name="homogeneous">False</property>
                <property name="selection-mode">none</property>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">x-office-calendar-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Daily Recommendation</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="daily_rec_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">emote-love-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Songs</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="heartbeat_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">audio-headphones-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">My Radio</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="radio_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">weather-overcast-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Cloud Music</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="cloud_disk_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">media-optical-bd-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Album</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="collection_album_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">vertical</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkBox">
                                <property name="width-request">140</property>
                                <property name="height-request">140</property>
                                <style>
                                    <class name="songlist-card-cover" />
                                    <class name="quick-entry-cover" />
                                </style>
                                <child>
                                    <object class="GtkImage">
                                        <property name="icon-name">non-starred-symbolic</property>
                                        <property name="pixel-size">64</property>
                                        <property name="hexpand">true</property>
                                        <property name="vexpand">true</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Song List</property>
                                <property name="justify">center</property>
                                <property name="max-width-chars">14</property>
                                <property name="ellipsize">end</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkGestureClick">
                                <signal name="pressed" handler="collection_songlist_cb" swapped="true" />
                            </object>
                        </child>
                    </object>
                </child>
            </object>
        </child>
        <child>
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <property name="hexpand">true</property>
                <property name="vexpand">true</property>
                <child>
                    <object class="GtkBox">
                        <property name="orientation">horizontal</property>
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">media-optical-cd-audio-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="halign">start</property>
                                <property name="label" translatable="yes">Recommended Song List</property>
                                <style>
                                    <class name="title-3" />
                                </style>
                            </object>
                        </child>
                    </object>
                </child>
                <child>
                    <object class="GtkSeparator">
                        <property name="sensitive">False</property>
                        <property name="can_focus">False</property>
                    </object>
                </child>
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
            </object>
        </child>
    </template>
</interface>
```

要点：6 个 `AdwAvatar` 改为 140×140 图标卡片（复用 `.songlist-card-cover` 圆角阴影 + `.quick-entry-cover` 强调色淡底）；`size=15000` 手写属性全清，分区标题换 `title-3`；`rec_grid` 外的 `AdwClamp(1000)` 去掉实现流式；六个 handler 与图标、标签文本与原模板一一对应。

- [ ] **Step 2: my_page.rs 移除头像子控件与日期逻辑**

`src/gui/my_page.rs` 的 `imp::MyPage` 结构体（:65-72）删除 `daily_rec_avatar` 字段，改为：

```rust
    pub struct MyPage {
        #[template_child]
        pub rec_grid: TemplateChild<gtk::FlowBox>,

        pub sender: OnceCell<Sender<Action>>,
    }
```

`daily_rec_cb`（:92-104）改为：

```rust
        #[template_callback]
        fn daily_rec_cb(&self) {
            let sender = self.sender.get().unwrap();
            sender.send_blocking(Action::ToMyPageDailyRec).unwrap();
        }
```

`ObjectImpl`（:137-149）改为：

```rust
    impl ObjectImpl for MyPage {}
```

- [ ] **Step 3: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 均成功，无 warning（重点确认无 `daily_rec_avatar` 相关残留引用）。

- [ ] **Step 4: Commit**

```bash
git add data/gtk/my-page.ui src/gui/my_page.rs
git commit -m "feat: 我的页统一——快捷入口改标准卡片网格，分区标题与间距规范化"
```

---

### Task 5: 全量验证与收尾

**Files:**
- 无新增改动（仅验证；若 fmt 有改动则一并提交）

**Interfaces:**
- Consumes: Task 1-4 的全部产出。

- [ ] **Step 1: 格式化与静态检查**

```bash
cargo fmt
cargo clippy 2>&1 | tail -10
```

Expected: clippy 无新增 warning（既有历史 warning 不在本轮处理范围）。

- [ ] **Step 2: 全量构建与数据文件校验**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5 && meson test 2>&1 | tail -10
```

Expected: 构建零警告；`meson test` 三项数据文件校验（desktop / metainfo / gschema）全部通过。

- [ ] **Step 3: 运行冒烟（有显示环境时）**

```bash
make run
```

或已安装环境下：

```bash
RUST_LOG=debug ./_build/src/netease-cloud-music-gtk4 2>&1 | grep -i critical
```

Expected: 无新增 GTK critical；人工核对矩阵：明/暗主题 × 窄(750px)/默认(1160px)/全屏 × 三页（卡片列数随宽度变化、Banner 铺满且高度 320、悬停态、分区标题样式三页一致）。无显示环境则跳过并在提交信息/汇报中注明「未做 GUI 人工验证」。

- [ ] **Step 4: Commit（仅当 fmt 产生改动）**

```bash
git add -u
git commit -m "style: cargo fmt"
```
