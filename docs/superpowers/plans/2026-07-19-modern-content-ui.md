# 展示内容 UI 现代化重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将应用全部展示内容 UI（歌曲行、歌单卡片、详情页头部、发现页分区）升级为现代化 Adwaita 原生风。

**Architecture:** 新增集中式 `data/themes/modern.css` 承载全部视觉规则（display 级 provider，在 `window.rs` 的 `class_init` 挂载）；`.ui` 模板只做样式类与局部结构调整；Rust 侧仅改 `songlist_grid_item.rs`、`songlist_row.rs`、`songlist_page.rs`、`toplist.rs`、`discover.rs`、`window.rs`。零新增 Action，不动 ListBox 渲染架构。

**Tech Stack:** Rust (gtk-rs 0.11 / libadwaita 0.9 v1_6)、GTK4 Builder 模板、GTK CSS、Meson + Cargo。

**设计规格：** `docs/superpowers/specs/2026-07-19-modern-content-ui-design.md`

## Global Constraints

- 项目无 Rust 测试框架（无 `#[test]`、无 dev-dependencies）。每个任务的"验证"= `cargo build` 零警告 + `cd _build && ninja` 通过（gresource 编译能抓出 ui/css 语法错误）。**不要**新增测试依赖。
- 所有颜色只允许引用 Libadwaita 命名色（`@accent_bg_color`、`@accent_fg_color`、`@theme_fg_color` 等），禁止硬编码十六进制颜色，保证明暗主题自适应。
- 新增 `.css` 必须登记到 `data/netease_cloud_music_gtk4.gresource.xml`。
- 新增用户可见字符串必须用 `translatable="yes"` 或 `gettext()` 包裹，并补 `po/zh_CN.po` 翻译。
- 源文件头部保留既有版权注释块风格；Rust 4 空格缩进，改完跑 `cargo fmt`。
- 不新增任何 `Action` 枚举成员；不改动 `src/application.rs`。
- 构建验证命令：`cargo build`（需 `_build` 已 meson setup 过，生成过 `src/config.rs`）；gresource 验证：`cd _build && ninja`。
- 不要提交 `_build/`、`target/`、`src/config.rs`（均已被 gitignore）。

## 与规格的两处细化（执行时以此为准）

1. 卡片封面与详情页封面**保留 `gtk::Image`**（不迁 `gtk::Picture`）：`GridView` 工厂路径靠 `paintable` 属性绑定、`FlowBox` 路径靠 `set_from_file`，`Image` 两条路都现成；圆角与阴影用 CSS `border-radius`/`box-shadow` 实现，视觉效果相同且零绑定风险。
2. 卡片上的悬停播放浮层是**纯视觉徽标**（`GtkImage`，非 `GtkButton`）：用户已确认点击行为=进详情页，而卡片整卡激活（FlowBox `child-activated` / GridView `activate`）已覆盖该行为；嵌套真按钮会吞掉点击事件、破坏现有激活链路且需键盘可达性处理，无任何行为收益。

---

### Task 1: modern.css 基础设施

**Files:**
- Create: `data/themes/modern.css`
- Modify: `data/netease_cloud_music_gtk4.gresource.xml`
- Modify: `src/window.rs`（`class_init` 附近，约 :116-119）

**Interfaces:**
- Produces: CSS 资源路径 `/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/modern.css`，全局 display 级 provider（`STYLE_PROVIDER_PRIORITY_APPLICATION`）。后续所有任务的样式规则都追加到 `data/themes/modern.css`。

- [ ] **Step 1: 创建 modern.css（先只放文件头与注释，规则由后续任务追加）**

```css
/*
 * modern.css
 * Copyright (C) 2026 gmg137 <gmg137 AT live.com>
 *
 * Distributed under terms of the GPL-3.0-or-later license.
 *
 * 展示内容 UI 现代化样式：歌曲行、歌单卡片、详情页头部、发现页。
 * 只允许引用 Libadwaita 命名色，禁止硬编码颜色值。
 */
```

- [ ] **Step 2: 登记 gresource**

`data/netease_cloud_music_gtk4.gresource.xml`，在 `<file compressed="true">themes/discover.css</file>` 行后追加：

```xml
        <file compressed="true">themes/modern.css</file>
```

- [ ] **Step 3: window.rs 全局挂载 provider**

`src/window.rs` 中 `NeteaseCloudMusicGtk4Window` 的 `class_init`（约 :116-119）改为：

```rust
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            load_css();
        }
```

文件末尾追加（模仿 `src/gui/discover.rs:285-296` 的 `load_css`）：

```rust
fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_resource("/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/modern.css");

    // Add the provider to the default screen
    style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
```

`CssProvider` 与 `style_context_add_provider_for_display` 由文件已有的 `gtk::{prelude::*, subclass::prelude::*, *}` 通配导入提供；若编译报未解析，补 `use gtk::{CssProvider, style_context_add_provider_for_display};`。

- [ ] **Step 4: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 均成功，无 warning；ninja 重新编译 gresource 不报错。

- [ ] **Step 5: Commit**

```bash
git add data/themes/modern.css data/netease_cloud_music_gtk4.gresource.xml src/window.rs
git commit -m "feat: 新增 modern.css 全局现代化样式基础设施"
```

---

### Task 2: 歌曲列表行现代化

**Files:**
- Modify: `data/gtk/songlist-row.ui`
- Modify: `src/gui/songlist_row.rs`（`switch_image`，:58-61）
- Modify: `data/themes/modern.css`
- Modify: `data/themes/discover.css`（删除迁移走的 `.song_row` 规则，:43-49）

**Interfaces:**
- Consumes: Task 1 的 modern.css 全局 provider。
- Produces: 样式类 `.song_row`（已有）、`.playing`（播放中行，新增）、`.row-actions`（行尾按钮组容器，新增）。`SonglistRow::switch_image(visible: bool)` 签名不变，内部新增 `playing` 类切换——`songlist_view.rs` 的全部调用点（:76-89、:128-139、:203-216）零改动。

- [ ] **Step 1: 调整 songlist-row.ui**

三处修改：

(a) 删除模板根部的行高写死属性（:7）：

```xml
        <property name="height-request">59</property>
```

（整行删除。）

(b) 手势限制主键（:136-138），在 `GtkGestureClick` 上加 `button` 属性：

```xml
            <object class="GtkGestureClick">
                <property name="button">1</property>
                <signal name="released" handler="on_click" swapped="yes" />
            </object>
```

(c) 行尾三按钮包进一个容器以便整体控制透明度。把 `like_button`、`album_button`、`remove_button` 三个 `<object class="GtkButton">…</object>` 块（:95-130）包进：

```xml
                        <child>
                            <object class="GtkBox">
                                <property name="halign">end</property>
                                <property name="valign">center</property>
                                <style>
                                    <class name="row-actions" />
                                </style>
                                <!-- 原有 like_button / album_button / remove_button 三个 child 原样移入此处 -->
                            </object>
                        </child>
```

三个按钮的 id、signal、style 全部原样保留，只改变嵌套层级。

- [ ] **Step 2: songlist_row.rs 播放态样式类切换**

`switch_image`（:58-61）改为：

```rust
    pub fn switch_image(&self, visible: bool) {
        let imp = self.imp();
        imp.play_icon.set_visible(visible);
        if visible {
            self.add_css_class("playing");
        } else {
            self.remove_css_class("playing");
        }
    }
```

- [ ] **Step 3: modern.css 追加歌曲行规则**

```css
/* ===== 歌曲列表行 ===== */

/* 行高由原模板写死 59px 改为 padding 控制（约 44px），提高列表密度 */
.song_row {
    padding-top: 6px;
    padding-bottom: 6px;
    border-radius: 8px;
}

.song_row:hover {
    background-color: alpha(@theme_fg_color, 0.06);
}

/* 播放中的行：歌名用强调色 */
.song_row.playing #title_label,
.song_row.playing #play_icon {
    color: @accent_bg_color;
}

/* 无版权置灰（从 discover.css 迁入，行为不变） */
.song_row label {
    opacity: 0.5;
}

.song_row.activatable label {
    opacity: 1;
}

/* 行尾操作按钮：默认隐藏，悬停淡入 */
.song_row .row-actions {
    opacity: 0;
    transition: opacity 150ms ease-out;
}

.song_row:hover .row-actions,
.song_row .row-actions:focus-within {
    opacity: 1;
}
```

注意：`.song_row.playing #title_label` 用的是模板内 widget 的 `name` 属性选择器。GTK CSS 中模板子控件以 `#<id>` 选择——若实测不生效（GTK4 对模板内部 id 选择器支持依版本有差异），备选方案：在 `switch_image` 里改为 `imp.title_label.add_css_class("playing-title")` / `remove_css_class`，CSS 相应改成 `.playing-title { color: @accent_bg_color; }`。实现时先试 `#title_label` 选择器，不行再落到 label 直挂类。

- [ ] **Step 4: discover.css 删除已迁移的置灰规则**

删除 `data/themes/discover.css` 末尾（:43-49）：

```css
.song_row label {
	opacity: 0.5;
}

.song_row.activatable label {
	opacity: 1;
}
```

- [ ] **Step 5: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 编译通过无 warning；gresource 重编译无 XML 错误。

- [ ] **Step 6: Commit**

```bash
git add data/gtk/songlist-row.ui src/gui/songlist_row.rs data/themes/modern.css data/themes/discover.css
git commit -m "feat: 歌曲列表行现代化——紧凑行高、悬停按钮、播放态高亮"
```

---

### Task 3: 歌单卡片重写（悬停播放徽标 + 圆角阴影封面）

**Files:**
- Modify: `src/gui/songlist_grid_item.rs`（`create()`，:54-97）
- Modify: `data/themes/modern.css`

**Interfaces:**
- Consumes: Task 1 的 provider。
- Produces: `SongListGridItem::create(pic_size: i32) -> (Box, Image, Label, Label)` **签名与返回类型不变**（`box_update_songlist` :99-126 与 `setup_factory` :134-182 全部调用点零改动）。新增样式类 `.songlist-card-cover`（封面 Image）、`.card-play-badge`（悬停播放徽标）。`setup_factory` 的 `connect_bind` 靠 `first_child`/`next_sibling` 遍历（:154-157）：卡片 Box 第一个孩子从 `GtkFrame` 变为 `GtkOverlay`，需同步微调取值链（见 Step 2）。

- [ ] **Step 1: 重写 create() 的封面部分**

`src/gui/songlist_grid_item.rs` 的 `create()`（:54-97）中，把 Frame 段替换为 Overlay 结构，函数其余部分（两个 Label 与返回值）不变：

```rust
    fn create(pic_size: i32) -> (Box, Image, Label, Label) {
        let boxs = Box::new(Orientation::Vertical, 0);

        let image = Image::builder()
            .pixel_size(pic_size)
            .icon_name("image-missing")
            .css_classes(vec!["songlist-card-cover".to_string()])
            .build();

        // 悬停时浮现的播放徽标（纯视觉，点击行为仍由整卡激活处理）
        let badge = Image::builder()
            .icon_name("media-playback-start-symbolic")
            .halign(Align::End)
            .valign(Align::End)
            .css_classes(vec!["card-play-badge".to_string()])
            .build();

        let overlay = Overlay::builder()
            .halign(Align::Center)
            .valign(Align::Center)
            .child(&image)
            .build();
        overlay.add_overlay(&badge);

        boxs.append(&overlay);
```

（其后 `label`、`label_author` 的构建代码与 `boxs.append(&label); boxs.append(&label_author); (boxs, image, label, label_author)` 原样保留。）

- [ ] **Step 2: 同步 setup_factory 的遍历链**

`create()` 第一个孩子由 Frame 变为 Overlay，但 `connect_bind`（:154-157）取的是 `frame.first_child()` 作为 image——`GtkOverlay` 的 `first_child()` 同样是其 `child`（即 image），**行为不变，无需改代码**。实现者须确认这一点：Overlay 的第一个 child 是 `child` 属性指向的 image，badge 通过 `add_overlay` 挂在其后。若实测绑定失效，把 :154-155 显式改为：

```rust
            let overlay = list_item.child().unwrap().first_child().unwrap();
            let image = overlay.first_child().unwrap();
```

- [ ] **Step 3: modern.css 追加卡片规则**

```css
/* ===== 歌单卡片 ===== */

.songlist-card-cover {
    border-radius: 12px;
    box-shadow: 0 1px 4px alpha(black, 0.18);
}

/* 悬停播放徽标：默认透明，悬停淡入 */
.card-play-badge {
    color: @accent_fg_color;
    background-color: @accent_bg_color;
    border-radius: 50%;
    padding: 8px;
    margin: 8px;
    opacity: 0;
    transition: opacity 150ms ease-out;
}

flowboxchild:hover .card-play-badge,
gridview > child:hover .card-play-badge {
    opacity: 1;
}
```

（GTK CSS 支持 `alpha(black, 0.18)` 与 `50%` 圆角；`black` 是命名色常量、非主题变量，阴影用它不破坏明暗主题。）

- [ ] **Step 4: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 通过，无 warning。

- [ ] **Step 5: Commit**

```bash
git add src/gui/songlist_grid_item.rs data/themes/modern.css
git commit -m "feat: 歌单卡片封面圆角阴影与悬停播放徽标"
```

---

### Task 4: 歌单/专辑详情页头部

**Files:**
- Modify: `data/gtk/songlist-page.ui`
- Modify: `src/gui/songlist_page.rs`（`init_songlist_info` :43-83、`init_songlist` :85-144、imp 模板子控件 :157 附近）
- Modify: `data/themes/modern.css`
- Modify: `po/zh_CN.po`（新增 "Play All" 翻译）

**Interfaces:**
- Consumes: Task 1 provider；`ncm_api::PlayListDetail.description: String` 与 `ncm_api::AlbumDetail.description: String`（crate 内已存在，见 model.rs :566/:627 附近，`Radio` 变体无简介）。
- Produces: 新模板子控件 `description_label: TemplateChild<Label>`；样式类 `.songlist-cover-frame`（封面圆角阴影）。`play_button` id 与 `play_button_clicked_cb` 不变。

- [ ] **Step 1: 改 songlist-page.ui 头部**

(a) 封面 Frame 加样式类、像素尺寸 140→200（:25-34）：

```xml
                            <object class="GtkFrame">
                                <style>
                                    <class name="songlist-cover-frame" />
                                </style>
                                <child>
                                    <object class="GtkImage" id="cover_image">
                                        <property name="halign">fill</property>
                                        <property name="valign">fill</property>
                                        <property name="pixel-size">200</property>
                                        <property name="icon-name">image-missing-symbolic</property>
                                    </object>
                                </child>
                            </object>
```

(b) 标题去掉 27 字符截断、改 2 行换行（:49-58）：

```xml
                                            <object class="GtkLabel" id="title_label">
                                                <property name="label">Title</property>
                                                <property name="halign">start</property>
                                                <property name="valign">end</property>
                                                <property name="wrap">True</property>
                                                <property name="lines">2</property>
                                                <property name="ellipsize">end</property>
                                                <style>
                                                    <class name="title-1" />
                                                </style>
                                            </object>
```

(c) `num_label` 之后（:74 的 `</object>` 后）插入简介行：

```xml
                                        <child>
                                            <object class="GtkLabel" id="description_label">
                                                <property name="halign">start</property>
                                                <property name="valign">end</property>
                                                <property name="visible">False</property>
                                                <property name="wrap">True</property>
                                                <property name="lines">2</property>
                                                <property name="ellipsize">end</property>
                                                <property name="xalign">0</property>
                                                <style>
                                                    <class name="dim-label" />
                                                    <class name="caption" />
                                                </style>
                                            </object>
                                        </child>
```

(d) 播放按钮改胶囊主按钮（:84-95），图标+文字：

```xml
                                            <object class="GtkButton" id="play_button">
                                                <property name="halign">end</property>
                                                <property name="valign">center</property>
                                                <signal name="clicked" handler="play_button_clicked_cb" swapped="true" />
                                                <property name="tooltip-text" translatable="yes">Play songs list</property>
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
```

注意：删掉原 `hexpand=true`（原用于把 like 顶到右边）；为保持布局，like_button 保持原位即可，两个按钮现在都在 `valign=end` 的右下按钮组内。若实测 like 按钮被挤到标题下方，给 play_button 的父 Box（:78-82）保持 `halign=fill` 不动即可，无需额外调整。

- [ ] **Step 2: songlist_page.rs 绑定简介与封面尺寸**

(a) imp 结构体（:157 附近 `#[template_child]` 区）追加：

```rust
        #[template_child]
        pub description_label: TemplateChild<Label>,
```

(b) `init_songlist_info` 中封面下载尺寸 (140,140)→(200,200)（:68）：

```rust
            cover_image.set_from_net(songlist.cover_img_url.to_owned(), path, (200, 200), sender);
```

(c) `init_songlist` 的 match 中补简介绑定。`SongListDetail::Album` 分支（:92 起）内 `imp.time_label.set_label(...)` 之后追加：

```rust
                let desc = detail.description.trim();
                imp.description_label.set_visible(!desc.is_empty());
                imp.description_label.set_label(desc);
```

`SongListDetail::PlayList` 分支（:109 起）的 `imp.num_label.set_label(...)` 之后追加：

```rust
                let desc = _detail.description.trim();
                imp.description_label.set_visible(!desc.is_empty());
                imp.description_label.set_label(desc);
```

（注意该分支原绑定是 `_detail`，下划线前缀要去掉——变量现在被使用了。）

`SongListDetail::Radio` 分支追加：

```rust
                imp.description_label.set_visible(false);
```

（d) `init_songlist_info` 里歌曲数清零处（:77-79）之后，重置简介避免上个歌单残留：

```rust
        imp.description_label.set_visible(false);
```

- [ ] **Step 3: modern.css 追加封面规则**

```css
/* ===== 详情页头部 ===== */

.songlist-cover-frame {
    border-radius: 12px;
    box-shadow: 0 2px 8px alpha(black, 0.22);
    padding: 0;
}

.songlist-cover-frame image {
    border-radius: 12px;
}
```

- [ ] **Step 4: po/zh_CN.po 补翻译**

在 `po/zh_CN.po` 末尾追加（参考文件内既有条目格式）：

```po
#: data/gtk/songlist-page.ui
msgid "Play All"
msgstr "播放全部"
```

- [ ] **Step 5: 构建验证**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
```

Expected: 通过。若 ninja 报 `AdwButtonContent` 未识别，确认根 `meson.build` 的 libadwaita 依赖 ≥1.4（项目要求 ≥1.5，应无问题）。

- [ ] **Step 6: Commit**

```bash
git add data/gtk/songlist-page.ui src/gui/songlist_page.rs data/themes/modern.css po/zh_CN.po
git commit -m "feat: 歌单详情页头部现代化——大封面、简介、播放全部主按钮"
```

---

### Task 5: 榜单页头部统一

**Files:**
- Modify: `data/gtk/toplist.ui`
- Modify: `src/gui/toplist.rs`（:58、:74 两处 `set_from_net`）

**Interfaces:**
- Consumes: Task 1 provider、Task 4 的 `.songlist-cover-frame` 样式。
- Produces: 无新接口；`play_button_clicked_cb`、`cover_image` 等 id 不变。

- [ ] **Step 1: 改 toplist.ui 头部**

(a) 封面 Frame 加样式类、尺寸 140→200（:45-54），与 Task 4 Step 1(a) 完全相同的改法。

(b) 标题去截断改 2 行（:63-72）：去掉 `<property name="max-width-chars">27</property>`，加 `<property name="wrap">True</property>` 与 `<property name="lines">2</property>`。

(c) 播放按钮改胶囊（:88-98）：

```xml
                                                    <object class="GtkButton" id="play_button">
                                                        <property name="halign">end</property>
                                                        <property name="valign">center</property>
                                                        <signal name="clicked" handler="play_button_clicked_cb" swapped="true" />
                                                        <property name="tooltip-text" translatable="yes">Play songs list</property>
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
```

（榜单页无简介数据——`TopList` 结构无 description 字段，头部不加简介行。）

- [ ] **Step 2: toplist.rs 封面下载尺寸**

`init_sidebar`（:58）与初始封面加载（:74）两处 `(140, 140)` 改为 `(200, 200)`：

```rust
        image.set_from_net(t.cover.to_owned(), path, (200, 200), sender);
```

```rust
        imp.cover_image
            .set_from_net(t.cover.to_owned(), path, (200, 200), sender);
```

（sidebar 行内 40px 小图不受下载尺寸影响，`set_pixel_size(40)` 在下载后设置，逻辑不变。）

- [ ] **Step 3: po/zh_CN.po 注释补充**

"Play All" 条目已存在（Task 4 添加），把参考注释更新为：

```po
#: data/gtk/songlist-page.ui data/gtk/toplist.ui
msgid "Play All"
msgstr "播放全部"
```

- [ ] **Step 4: 构建验证 + Commit**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
git add data/gtk/toplist.ui src/gui/toplist.rs po/zh_CN.po
git commit -m "feat: 榜单页头部与详情页统一现代化"
```

---

### Task 6: 发现页分区标题与轮播微调

**Files:**
- Modify: `data/gtk/discover.ui`
- Modify: `src/gui/discover.rs`（banner Picture 创建处，:83-110）
- Modify: `data/themes/modern.css`
- Modify: `data/themes/discover.css`（删除迁移走的 `flowboxchild` 规则，:17-28）

**Interfaces:**
- Consumes: Task 1 provider。
- Produces: 样式类 `.discover-section-title`（分区标题，可空——直接用 Libadwaita 内建 `title-3`）、`.banner-image`（轮播图圆角阴影）。

- [ ] **Step 1: discover.ui 分区标题换标准样式类**

两处分区标题（"Top Picks" :118-127、"New Albums" :185-194）：删除 `<attributes>` 块，改用样式类：

```xml
                                    <object class="GtkLabel">
                                        <property name="halign">start</property>
                                        <property name="valign">center</property>
                                        <property name="margin-start">9</property>
                                        <property name="label" translatable="yes">Top Picks</property>
                                        <style>
                                            <class name="title-3" />
                                        </style>
                                    </object>
```

（"New Albums" 同理，label 文本不同。）

- [ ] **Step 2: discover.rs banner 加样式类**

banner 的 `gtk::Picture` 创建处（:83-110 的 builder 链）加：

```rust
            .css_classes(vec!["banner-image".to_string()])
```

（保留既有 `can_shrink` 等属性不动。实现者先读该段实际 builder 写法，把 css_classes 追加进 builder 链。）

- [ ] **Step 3: modern.css 追加发现页规则**

```css
/* ===== 发现页 ===== */

.banner-image {
    border-radius: 16px;
    box-shadow: 0 2px 8px alpha(black, 0.18);
}

/* FlowBox 卡片间距（从 discover.css 迁入，统一管理；规则内容不变） */
flowboxchild {
  margin: 12px;
  padding: 4px;
}

flowboxchild:hover {
    background-color: alpha(@theme_fg_color, 0.1);
}

flowboxchild:active {
    background-color: alpha(@theme_fg_color, 0.2);
}
```

同时删除 `data/themes/discover.css` 中已迁移的 `flowboxchild` 三条规则（:17-28），避免两处维护。

- [ ] **Step 4: 构建验证 + Commit**

```bash
cargo build 2>&1 | tail -5
cd _build && ninja 2>&1 | tail -5
git add data/gtk/discover.ui src/gui/discover.rs data/themes/modern.css data/themes/discover.css
git commit -m "feat: 发现页分区标题标准化、轮播图圆角与卡片间距迁移"
```

---

### Task 7: 全量验证与收尾

**Files:**
- 无新改动（仅验证；若发现问题回到对应任务修复）

**Interfaces:**
- Consumes: Task 1-6 全部成果。

- [ ] **Step 1: 完整构建与格式化检查**

```bash
cargo fmt --check
cargo build 2>&1 | grep -E "warning|error" | head -20
cd _build && ninja && meson test
```

Expected: fmt 无 diff；构建零 warning；meson test（desktop/metainfo/gschema 校验）全过。

- [ ] **Step 2: 运行应用手工验证**

```bash
RUST_LOG=netease_cloud_music_gtk4 ./_build/src/netease-cloud-music-gtk4
```

逐项过清单（明暗主题各一遍，主题在主菜单 ThemeSelector 切换）：

- 发现页：卡片封面圆角阴影、悬停浮现播放徽标、点击卡片（含徽标位置）进详情页、轮播圆角正常、分区标题为 title-3 样式
- 榜单页：切榜正常、行高变紧凑、悬停行有背景且行尾按钮淡入、点击播放后该行歌名变强调色
- 歌单详情页：200px 圆角封面、标题 2 行、简介 2 行（空简介的歌单不占空间）、「播放全部」胶囊按钮可播放
- 搜索页（歌曲/歌单/歌手）：行样式与卡片样式均生效
- 我的页：歌曲列表行样式生效
- 终端无新增 GTK critical/warning 输出

- [ ] **Step 3: 更新 AGENTS.md（样式文件清单）**

`AGENTS.md` 的「代码结构」中 `data/themes/*.css` 描述行补充 modern.css 的存在与职责（集中式现代化展示样式），保持文档与代码同步。

- [ ] **Step 4: 最终 Commit**

```bash
git add AGENTS.md
git commit -m "docs: AGENTS.md 补充 modern.css 说明"
```
