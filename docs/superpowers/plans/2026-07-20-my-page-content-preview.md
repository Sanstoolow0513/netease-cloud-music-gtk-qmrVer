# “我的”页内容预览重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将登录后的“我的”页改为每日推荐、收藏歌曲、收藏专辑、收藏歌单四个可直接浏览的预览分区，并保留进入完整列表的“更多”导航。

**Architecture:** `MyPage` 持有四个独立状态区，歌曲预览复用紧凑模式的 `SonglistRow`，专辑和歌单预览复用 `SongListGridItem`。`Action` 总线按 `MyPageSection` 独立加载四类数据，`window.rs` 负责收藏状态和视图桥接；现有四个完整页 Action 保持为“更多”目标。

**Tech Stack:** Rust 2024、gtk-rs 0.11、GTK4 CompositeTemplate、libadwaita 0.9、GLib MainContext、async-channel、gettext、Meson + Cargo。

**设计规格：** `docs/superpowers/specs/2026-07-20-my-page-content-preview-design.md`

## Global Constraints

- 首页仅保留四个分区，顺序固定为：每日推荐、收藏歌曲、收藏专辑、收藏歌单。
- 预览上限固定为歌曲 8/8、专辑 10、歌单 10；收藏歌单必须安全跳过第一个“喜欢的音乐”歌单。
- 宽窗口歌曲为两列、每列连续 4 首；窄窗口自动叠为单列，顺序保持 1–8。
- 四个分区独立加载、独立失败、独立重试；不得恢复当前无限自动重试。
- “更多”继续复用 `ToMyPageDailyRec`、`ToMyPageHeartbeat`、`ToMyPageAlbums`、`ToMyPageSonglist`。
- 不新增第三方依赖，不修改 `SearchType`、音频核心、GSettings、API 封装和 gresource 清单。
- 用户可见字符串必须使用 GTK `translatable="yes"` 或 `gettext()`；复用已有 `View More`，新增 `No content`、`Failed to load`、`Retry`。
- 新样式只能使用现有 `modern.css` 规则和 Libadwaita 命名色；本计划不新增 CSS。
- 保留项目既有版权头、Rust 四空格缩进；不要提交 `.cursor/`、`.superpowers/`、`CLAUDE.md`、`src/config.rs` 或构建目录。
- 项目当前无测试依赖；纯数据整形使用内联 `#[cfg(test)]` 单测，不新增 dev-dependencies。

---

### Task 1: 预览数据整形助手与边界测试

**Files:**
- Modify: `src/application.rs`（Action 定义之后、`mod imp` 之前；文件末尾测试模块）

**Interfaces:**
- Produces: `take_preview<T>(Vec<T>, usize) -> Vec<T>`
- Produces: `skip_liked_playlist<T>(Vec<T>, Option<usize>) -> Vec<T>`
- Produces: `MY_PAGE_SONG_PREVIEW_LIMIT = 8`
- Produces: `MY_PAGE_COLLECTION_PREVIEW_LIMIT = 10`
- Consumes: 无

- [ ] **Step 1: 先写失败测试**

在 `src/application.rs` 文件末尾增加：

```rust
#[cfg(test)]
mod tests {
    use super::{
        MY_PAGE_COLLECTION_PREVIEW_LIMIT, MY_PAGE_SONG_PREVIEW_LIMIT, skip_liked_playlist,
        take_preview,
    };

    #[test]
    fn preview_helpers_limit_and_preserve_order() {
        assert_eq!(MY_PAGE_SONG_PREVIEW_LIMIT, 8);
        assert_eq!(MY_PAGE_COLLECTION_PREVIEW_LIMIT, 10);
        assert_eq!(take_preview(vec![1, 2, 3, 4], 3), vec![1, 2, 3]);
        assert_eq!(take_preview(vec![1, 2], 3), vec![1, 2]);
    }

    #[test]
    fn preview_helpers_skip_liked_playlist_safely() {
        assert_eq!(skip_liked_playlist::<i32>(vec![], Some(10)), vec![]);
        assert_eq!(skip_liked_playlist(vec![0], Some(10)), vec![]);
        assert_eq!(
            skip_liked_playlist(vec![0, 1, 2, 3], Some(2)),
            vec![1, 2]
        );
        assert_eq!(
            skip_liked_playlist(vec![0, 1, 2, 3], None),
            vec![1, 2, 3]
        );
    }
}
```

- [ ] **Step 2: 运行测试并确认先失败**

Run:

```bash
cargo test preview_helpers -- --nocapture
```

Expected: 编译失败，指出 `take_preview` 和 `skip_liked_playlist` 尚未定义。

- [ ] **Step 3: 添加最小实现**

在 `Action` 枚举结束后、`mod imp` 之前增加：

```rust
const MY_PAGE_SONG_PREVIEW_LIMIT: usize = 8;
const MY_PAGE_COLLECTION_PREVIEW_LIMIT: usize = 10;

fn take_preview<T>(items: Vec<T>, limit: usize) -> Vec<T> {
    items.into_iter().take(limit).collect()
}

fn skip_liked_playlist<T>(items: Vec<T>, limit: Option<usize>) -> Vec<T> {
    match limit {
        Some(limit) => items.into_iter().skip(1).take(limit).collect(),
        None => items.into_iter().skip(1).collect(),
    }
}
```

- [ ] **Step 4: 运行测试并确认通过**

Run:

```bash
cargo test preview_helpers -- --nocapture
```

Expected: `2 passed; 0 failed`。

- [ ] **Step 5: Commit**

```bash
git add src/application.rs
git commit -m "test: 覆盖我的页预览截取边界"
```

---

### Task 2: SonglistRow 首页紧凑模式

**Files:**
- Modify: `src/gui/songlist_row.rs:70-84`

**Interfaces:**
- Produces: `SonglistRow::set_my_page_preview_mode(&self)`
- Consumes: 现有 `set_album_button_visible`、`set_remove_button_visible`

- [ ] **Step 1: 添加专用显示 API**

在 `set_remove_button_visible()` 之后增加：

```rust
    pub fn set_my_page_preview_mode(&self) {
        self.imp().album_label.set_visible(false);
        self.set_album_button_visible(false);
        self.set_remove_button_visible(false);
    }
```

该方法只由“我的”页新建的行调用；完整页未调用时保持现有默认行为。

- [ ] **Step 2: 格式化并编译**

Run:

```bash
cargo fmt
cargo build
```

Expected: 构建成功，无新增 warning。

- [ ] **Step 3: Commit**

```bash
git add src/gui/songlist_row.rs
git commit -m "feat: 为我的页增加紧凑歌曲行模式"
```

---

### Task 3: 四分区模板、视图状态与独立数据加载

**Files:**
- Modify: `src/model.rs`（`SearchType` 之前）
- Replace: `data/gtk/my-page.ui`
- Modify: `src/gui/my_page.rs`
- Modify: `src/window.rs:804-816`
- Modify: `src/application.rs:60-120, 305-365, 1118-1287`
- Modify: `po/zh_CN.po`

**Interfaces:**
- Produces: `MyPageSection::{DailyRec, FavoriteSongs, FavoriteAlbums, FavoriteSongLists}` 及 `MyPageSection::ALL`
- Produces Action:
  - `LoadMyPageSection(MyPageSection)`
  - `SetupMyPageSongs(MyPageSection, Vec<SongInfo>)`
  - `SetupMyPageCollections(MyPageSection, Vec<SongList>)`
  - `FailMyPageSection(MyPageSection)`
- Produces Window bridge:
  - `prepare_my_page()`
  - `set_my_page_section_loading(MyPageSection)`
  - `update_my_page_songs(MyPageSection, Vec<SongInfo>)`
  - `update_my_page_collections(MyPageSection, Vec<SongList>)`
  - `fail_my_page_section(MyPageSection)`
- Consumes: Task 1 的预览助手、Task 2 的紧凑歌曲行 API

- [ ] **Step 1: 定义分区枚举**

在 `src/model.rs` 的 `SearchType` 之前增加：

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MyPageSection {
    DailyRec,
    FavoriteSongs,
    FavoriteAlbums,
    FavoriteSongLists,
}

impl MyPageSection {
    pub const ALL: [Self; 4] = [
        Self::DailyRec,
        Self::FavoriteSongs,
        Self::FavoriteAlbums,
        Self::FavoriteSongLists,
    ];
}
```

- [ ] **Step 2: 替换 my-page.ui**

完整替换 `data/gtk/my-page.ui`。模板必须包含以下精确 ID：

- 状态栈：`daily_state`、`favorite_songs_state`、`albums_state`、`songlists_state`
- 更多按钮：`daily_more_button`、`favorite_songs_more_button`、`albums_more_button`、`songlists_more_button`
- 歌曲列表：`daily_left`、`daily_right`、`favorite_songs_left`、`favorite_songs_right`
- 收藏网格：`albums_grid`、`songlists_grid`

使用以下完整结构；四个 Stack 的页面名统一为 `loading`、`content`、`empty`、`error`：

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
            <object class="GtkBox">
                <property name="orientation">vertical</property>
                <property name="spacing">12</property>
                <child>
                    <object class="GtkBox">
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">x-office-calendar-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Daily Recommendation</property>
                                <style><class name="title-3" /></style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton" id="daily_more_button">
                                <property name="hexpand">true</property>
                                <property name="halign">end</property>
                                <property name="sensitive">false</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="daily_rec_cb" swapped="true" />
                                <style><class name="flat" /></style>
                            </object>
                        </child>
                    </object>
                </child>
                <child><object class="GtkSeparator" /></child>
                <child>
                    <object class="GtkStack" id="daily_state">
                        <property name="visible-child-name">loading</property>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">loading</property>
                                <property name="child">
                                    <object class="GtkSpinner">
                                        <property name="spinning">true</property>
                                        <property name="halign">center</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">content</property>
                                <property name="child">
                                    <object class="GtkFlowBox">
                                        <property name="selection-mode">none</property>
                                        <property name="min-children-per-line">1</property>
                                        <property name="max-children-per-line">2</property>
                                        <property name="homogeneous">true</property>
                                        <property name="column-spacing">16</property>
                                        <property name="row-spacing">16</property>
                                        <child>
                                            <object class="GtkListBox" id="daily_left">
                                                <property name="width-request">360</property>
                                                <property name="selection-mode">none</property>
                                                <style><class name="boxed-list" /></style>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkListBox" id="daily_right">
                                                <property name="width-request">360</property>
                                                <property name="selection-mode">none</property>
                                                <style><class name="boxed-list" /></style>
                                            </object>
                                        </child>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">empty</property>
                                <property name="child">
                                    <object class="GtkLabel">
                                        <property name="label" translatable="yes">No content</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <style><class name="dim-label" /></style>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">error</property>
                                <property name="child">
                                    <object class="GtkBox">
                                        <property name="halign">center</property>
                                        <property name="spacing">8</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <child>
                                            <object class="GtkLabel">
                                                <property name="label" translatable="yes">Failed to load</property>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkButton">
                                                <property name="label" translatable="yes">Retry</property>
                                                <signal name="clicked" handler="retry_daily_cb" swapped="true" />
                                            </object>
                                        </child>
                                    </object>
                                </property>
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
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">emote-love-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Songs</property>
                                <style><class name="title-3" /></style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton" id="favorite_songs_more_button">
                                <property name="hexpand">true</property>
                                <property name="halign">end</property>
                                <property name="sensitive">false</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="heartbeat_cb" swapped="true" />
                                <style><class name="flat" /></style>
                            </object>
                        </child>
                    </object>
                </child>
                <child><object class="GtkSeparator" /></child>
                <child>
                    <object class="GtkStack" id="favorite_songs_state">
                        <property name="visible-child-name">loading</property>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">loading</property>
                                <property name="child">
                                    <object class="GtkSpinner">
                                        <property name="spinning">true</property>
                                        <property name="halign">center</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">content</property>
                                <property name="child">
                                    <object class="GtkFlowBox">
                                        <property name="selection-mode">none</property>
                                        <property name="min-children-per-line">1</property>
                                        <property name="max-children-per-line">2</property>
                                        <property name="homogeneous">true</property>
                                        <property name="column-spacing">16</property>
                                        <property name="row-spacing">16</property>
                                        <child>
                                            <object class="GtkListBox" id="favorite_songs_left">
                                                <property name="width-request">360</property>
                                                <property name="selection-mode">none</property>
                                                <style><class name="boxed-list" /></style>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkListBox" id="favorite_songs_right">
                                                <property name="width-request">360</property>
                                                <property name="selection-mode">none</property>
                                                <style><class name="boxed-list" /></style>
                                            </object>
                                        </child>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">empty</property>
                                <property name="child">
                                    <object class="GtkLabel">
                                        <property name="label" translatable="yes">No content</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <style><class name="dim-label" /></style>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">error</property>
                                <property name="child">
                                    <object class="GtkBox">
                                        <property name="halign">center</property>
                                        <property name="spacing">8</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <child>
                                            <object class="GtkLabel">
                                                <property name="label" translatable="yes">Failed to load</property>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkButton">
                                                <property name="label" translatable="yes">Retry</property>
                                                <signal name="clicked" handler="retry_favorite_songs_cb" swapped="true" />
                                            </object>
                                        </child>
                                    </object>
                                </property>
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
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">media-optical-bd-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Album</property>
                                <style><class name="title-3" /></style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton" id="albums_more_button">
                                <property name="hexpand">true</property>
                                <property name="halign">end</property>
                                <property name="sensitive">false</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="collection_album_cb" swapped="true" />
                                <style><class name="flat" /></style>
                            </object>
                        </child>
                    </object>
                </child>
                <child><object class="GtkSeparator" /></child>
                <child>
                    <object class="GtkStack" id="albums_state">
                        <property name="visible-child-name">loading</property>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">loading</property>
                                <property name="child">
                                    <object class="GtkSpinner">
                                        <property name="spinning">true</property>
                                        <property name="halign">center</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">content</property>
                                <property name="child">
                                    <object class="GtkFlowBox" id="albums_grid">
                                        <property name="hexpand">true</property>
                                        <property name="valign">start</property>
                                        <property name="max-children-per-line">12</property>
                                        <property name="min-children-per-line">3</property>
                                        <property name="selection-mode">none</property>
                                        <property name="activate-on-single-click">true</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">empty</property>
                                <property name="child">
                                    <object class="GtkLabel">
                                        <property name="label" translatable="yes">No content</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <style><class name="dim-label" /></style>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">error</property>
                                <property name="child">
                                    <object class="GtkBox">
                                        <property name="halign">center</property>
                                        <property name="spacing">8</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <child>
                                            <object class="GtkLabel">
                                                <property name="label" translatable="yes">Failed to load</property>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkButton">
                                                <property name="label" translatable="yes">Retry</property>
                                                <signal name="clicked" handler="retry_albums_cb" swapped="true" />
                                            </object>
                                        </child>
                                    </object>
                                </property>
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
                        <property name="spacing">8</property>
                        <child>
                            <object class="GtkImage">
                                <property name="icon-name">non-starred-symbolic</property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkLabel">
                                <property name="label" translatable="yes">Favorite Song List</property>
                                <style><class name="title-3" /></style>
                            </object>
                        </child>
                        <child>
                            <object class="GtkButton" id="songlists_more_button">
                                <property name="hexpand">true</property>
                                <property name="halign">end</property>
                                <property name="sensitive">false</property>
                                <property name="icon-name">view-more-symbolic</property>
                                <property name="tooltip-text" translatable="yes">View More</property>
                                <signal name="clicked" handler="collection_songlist_cb" swapped="true" />
                                <style><class name="flat" /></style>
                            </object>
                        </child>
                    </object>
                </child>
                <child><object class="GtkSeparator" /></child>
                <child>
                    <object class="GtkStack" id="songlists_state">
                        <property name="visible-child-name">loading</property>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">loading</property>
                                <property name="child">
                                    <object class="GtkSpinner">
                                        <property name="spinning">true</property>
                                        <property name="halign">center</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">content</property>
                                <property name="child">
                                    <object class="GtkFlowBox" id="songlists_grid">
                                        <property name="hexpand">true</property>
                                        <property name="valign">start</property>
                                        <property name="max-children-per-line">12</property>
                                        <property name="min-children-per-line">3</property>
                                        <property name="selection-mode">none</property>
                                        <property name="activate-on-single-click">true</property>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">empty</property>
                                <property name="child">
                                    <object class="GtkLabel">
                                        <property name="label" translatable="yes">No content</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <style><class name="dim-label" /></style>
                                    </object>
                                </property>
                            </object>
                        </child>
                        <child>
                            <object class="GtkStackPage">
                                <property name="name">error</property>
                                <property name="child">
                                    <object class="GtkBox">
                                        <property name="halign">center</property>
                                        <property name="spacing">8</property>
                                        <property name="margin-top">24</property>
                                        <property name="margin-bottom">24</property>
                                        <child>
                                            <object class="GtkLabel">
                                                <property name="label" translatable="yes">Failed to load</property>
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkButton">
                                                <property name="label" translatable="yes">Retry</property>
                                                <signal name="clicked" handler="retry_songlists_cb" swapped="true" />
                                            </object>
                                        </child>
                                    </object>
                                </property>
                            </object>
                        </child>
                    </object>
                </child>
            </object>
        </child>
    </template>
</interface>
```

- [ ] **Step 3: 扩展 Action 协议**

将 `src/application.rs` 的 `// my` 段替换为：

```rust
    // my
    InitMyPage,
    LoadMyPageSection(MyPageSection),
    SetupMyPageSongs(MyPageSection, Vec<SongInfo>),
    SetupMyPageCollections(MyPageSection, Vec<SongList>),
    FailMyPageSection(MyPageSection),
```

旧的 `InitMyPageRecSongList(Vec<SongList>)` 必须删除。

- [ ] **Step 4: 重写 MyPage 的视图逻辑**

`src/gui/my_page.rs` 保留现有 wrapper/CompositeTemplate 模式，导入改为：

```rust
use async_channel::Sender;
use gio::Settings;
use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

use crate::{
    APP_ID,
    application::Action,
    gui::{SongListGridItem, songlist_row::SonglistRow},
    model::MyPageSection,
};
```

`impl MyPage` 必须提供以下方法：

```rust
    pub fn reset(&self) {
        let imp = self.imp();
        for list in [
            imp.daily_left.get(),
            imp.daily_right.get(),
            imp.favorite_songs_left.get(),
            imp.favorite_songs_right.get(),
        ] {
            Self::clear_listbox(&list);
        }
        SongListGridItem::box_clear(imp.albums_grid.get());
        SongListGridItem::box_clear(imp.songlists_grid.get());
        imp.albums.borrow_mut().clear();
        imp.songlists.borrow_mut().clear();
        imp.active_preview_row.replace(None);
        for section in MyPageSection::ALL {
            self.set_section_state(section, "loading");
        }
    }

    pub fn set_loading(&self, section: MyPageSection) {
        self.set_section_state(section, "loading");
    }

    pub fn set_failed(&self, section: MyPageSection) {
        self.set_section_state(section, "error");
    }

    pub fn update_songs(
        &self,
        section: MyPageSection,
        songs: &[SongInfo],
        likes: &[bool],
    ) {
        let imp = self.imp();
        let (left, right) = match section {
            MyPageSection::DailyRec => (imp.daily_left.get(), imp.daily_right.get()),
            MyPageSection::FavoriteSongs => (
                imp.favorite_songs_left.get(),
                imp.favorite_songs_right.get(),
            ),
            _ => return,
        };

        Self::clear_listbox(&left);
        Self::clear_listbox(&right);
        if songs.is_empty() {
            self.set_section_state(section, "empty");
            return;
        }

        let split = songs.len().min(4);
        left.set_visible(true);
        right.set_visible(songs.len() > split);
        self.fill_song_list(&left, &songs[..split], &likes[..split]);
        self.fill_song_list(&right, &songs[split..], &likes[split..]);
        self.set_section_state(section, "content");
    }

    pub fn update_collections(&self, section: MyPageSection, items: Vec<SongList>) {
        let imp = self.imp();
        let (grid, show_author) = match section {
            MyPageSection::FavoriteAlbums => (imp.albums_grid.get(), true),
            MyPageSection::FavoriteSongLists => (imp.songlists_grid.get(), false),
            _ => return,
        };

        SongListGridItem::box_clear(grid.clone());
        match section {
            MyPageSection::FavoriteAlbums => imp.albums.borrow_mut().clear(),
            MyPageSection::FavoriteSongLists => imp.songlists.borrow_mut().clear(),
            _ => unreachable!(),
        }
        if items.is_empty() {
            self.set_section_state(section, "empty");
            return;
        }

        let sender = imp.sender.get().unwrap();
        SongListGridItem::box_update_songlist(grid, &items, 140, show_author, sender);
        match section {
            MyPageSection::FavoriteAlbums => imp.albums.replace(items),
            MyPageSection::FavoriteSongLists => imp.songlists.replace(items),
            _ => unreachable!(),
        };
        self.set_section_state(section, "content");
    }

    fn fill_song_list(&self, list: &gtk::ListBox, songs: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let settings = imp.settings.get().unwrap();

        for (song, like_song) in songs.iter().zip(likes.iter()) {
            let row = SonglistRow::new(sender.clone(), song);
            row.set_property("like", like_song);
            row.set_my_page_preview_mode();
            settings
                .bind("not-ignore-grey", &row, "not-ignore-grey")
                .get_only()
                .build();

            let song = song.clone();
            gtk::prelude::ListBoxRowExt::connect_activate(
                &row,
                glib::clone!(
                    #[weak(rename_to = page)]
                    self,
                    #[strong]
                    sender,
                    move |row| {
                        page.activate_preview_row(row);
                        sender.send_blocking(Action::AddPlay(song.clone())).unwrap();
                    }
                ),
            );
            list.append(&row);
        }
    }

    fn activate_preview_row(&self, row: &SonglistRow) {
        let imp = self.imp();
        if let Some(old_row) = imp
            .active_preview_row
            .borrow()
            .as_ref()
            .and_then(|row| row.upgrade())
        {
            old_row.switch_image(false);
        }
        row.switch_image(true);
        imp.active_preview_row.replace(Some(row.downgrade()));
    }

    fn set_section_state(&self, section: MyPageSection, state: &str) {
        let imp = self.imp();
        let (stack, more_button) = match section {
            MyPageSection::DailyRec => (imp.daily_state.get(), imp.daily_more_button.get()),
            MyPageSection::FavoriteSongs => (
                imp.favorite_songs_state.get(),
                imp.favorite_songs_more_button.get(),
            ),
            MyPageSection::FavoriteAlbums => {
                (imp.albums_state.get(), imp.albums_more_button.get())
            }
            MyPageSection::FavoriteSongLists => (
                imp.songlists_state.get(),
                imp.songlists_more_button.get(),
            ),
        };
        stack.set_visible_child_name(state);
        more_button.set_sensitive(state == "content");
    }

    fn clear_listbox(list: &gtk::ListBox) {
        while let Some(child) = list.last_child() {
            list.remove(&child);
        }
    }
```

在 `imp::MyPage` 中把模板字段改为：

```rust
        #[template_child]
        pub daily_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub favorite_songs_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub albums_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub songlists_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub daily_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub favorite_songs_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub albums_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub songlists_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub daily_left: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub daily_right: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub favorite_songs_left: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub favorite_songs_right: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub albums_grid: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub songlists_grid: TemplateChild<gtk::FlowBox>,

        pub albums: RefCell<Vec<SongList>>,
        pub songlists: RefCell<Vec<SongList>>,
        pub active_preview_row: RefCell<Option<glib::WeakRef<SonglistRow>>>,
        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,
```

`ObjectImpl::constructed()` 只连接一次收藏网格，避免重试后重复注册 signal：

```rust
    impl ObjectImpl for MyPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.settings.set(Settings::new(APP_ID)).unwrap();
            let obj = self.obj();

            self.albums_grid.connect_child_activated(glib::clone!(
                #[weak]
                obj,
                move |_, child| {
                    let imp = obj.imp();
                    if let Some(item) = imp.albums.borrow().get(child.index() as usize) {
                        imp.sender
                            .get()
                            .unwrap()
                            .send_blocking(Action::ToAlbumPage(item.clone()))
                            .unwrap();
                    }
                }
            ));
            self.songlists_grid.connect_child_activated(glib::clone!(
                #[weak]
                obj,
                move |_, child| {
                    let imp = obj.imp();
                    if let Some(item) = imp.songlists.borrow().get(child.index() as usize) {
                        imp.sender
                            .get()
                            .unwrap()
                            .send_blocking(Action::ToSongListPage(item.clone()))
                            .unwrap();
                    }
                }
            ));
        }
    }
```

模板回调只保留四个“更多”并新增四个重试；删除电台和云盘回调：

```rust
        #[template_callback]
        fn daily_rec_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageDailyRec)
                .unwrap();
        }

        #[template_callback]
        fn heartbeat_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageHeartbeat)
                .unwrap();
        }

        #[template_callback]
        fn collection_album_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageAlbums)
                .unwrap();
        }

        #[template_callback]
        fn collection_songlist_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageSonglist)
                .unwrap();
        }

        #[template_callback]
        fn retry_daily_cb(&self) {
            self.load_section(MyPageSection::DailyRec);
        }

        #[template_callback]
        fn retry_favorite_songs_cb(&self) {
            self.load_section(MyPageSection::FavoriteSongs);
        }

        #[template_callback]
        fn retry_albums_cb(&self) {
            self.load_section(MyPageSection::FavoriteAlbums);
        }

        #[template_callback]
        fn retry_songlists_cb(&self) {
            self.load_section(MyPageSection::FavoriteSongLists);
        }

        fn load_section(&self, section: MyPageSection) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::LoadMyPageSection(section))
                .unwrap();
        }
```

- [ ] **Step 5: 将 Window 改为分区桥接**

删除旧 `init_my_page(Vec<SongList>)`，在 `src/window.rs` 同一位置增加：

```rust
    pub fn prepare_my_page(&self) {
        self.switch_my_page_to_login();
        self.imp().my_page.reset();
    }

    pub fn set_my_page_section_loading(&self, section: MyPageSection) {
        self.imp().my_page.set_loading(section);
    }

    pub fn update_my_page_songs(&self, section: MyPageSection, songs: Vec<SongInfo>) {
        let likes = self.get_song_likes(&songs);
        self.imp().my_page.update_songs(section, &songs, &likes);
    }

    pub fn update_my_page_collections(
        &self,
        section: MyPageSection,
        items: Vec<SongList>,
    ) {
        self.imp().my_page.update_collections(section, items);
    }

    pub fn fail_my_page_section(&self, section: MyPageSection) {
        self.imp().my_page.set_failed(section);
    }
```

- [ ] **Step 6: 调整登录初始化时序**

在 `Action::CheckLogin` 成功分支中，设置 uid/cookie 后立即执行：

```rust
window.prepare_my_page();
```

删除该分支直接发送的：

```rust
sender.send(Action::InitMyPage).await.unwrap();
```

把 `Action::InitUserInfo` 改为无论收藏 ID 请求成功或失败都启动首页加载：

```rust
            Action::InitUserInfo(login_info) => {
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match ncmapi.client.user_song_id_list(login_info.uid).await {
                        Ok(song_ids) => window.set_user_like_songs(&song_ids),
                        Err(err) => error!("{:?}", err),
                    }
                    if window.is_logined() {
                        sender.send(Action::InitMyPage).await.unwrap();
                    }
                });
            }
```

- [ ] **Step 7: 实现四类独立加载**

增加失败汇报助手：

```rust
fn fail_my_page_request(
    sender: &Sender<Action>,
    section: MyPageSection,
    err: impl std::fmt::Debug,
) {
    error!("{:?}", err);
    sender
        .send_blocking(Action::FailMyPageSection(section))
        .unwrap();
    sender
        .send_blocking(Action::AddToast(gettext(
            "Request for interface failed, please try again!",
        )))
        .unwrap();
}
```

用以下 match 分支替换旧 `InitMyPage` / `InitMyPageRecSongList`：

```rust
            Action::InitMyPage => {
                if window.is_logined() {
                    window.prepare_my_page();
                    for section in MyPageSection::ALL {
                        imp.sender
                            .send_blocking(Action::LoadMyPageSection(section))
                            .unwrap();
                    }
                }
            }
            Action::LoadMyPageSection(section) => {
                window.set_my_page_section_loading(section);
                let sender = imp.sender.clone();
                MAINCONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
                    match section {
                        MyPageSection::DailyRec => {
                            match ncmapi.client.recommend_songs().await {
                                Ok(songs) => {
                                    sender
                                        .send(Action::SetupMyPageSongs(
                                            section,
                                            take_preview(
                                                songs,
                                                MY_PAGE_SONG_PREVIEW_LIMIT,
                                            ),
                                        ))
                                        .await
                                        .unwrap();
                                }
                                Err(err) => fail_my_page_request(&sender, section, err),
                            }
                        }
                        MyPageSection::FavoriteSongs => {
                            match ncmapi.client.user_song_list(window.get_uid(), 0, 1).await {
                                Ok(songlists) => {
                                    if let Some(songlist) = songlists.first() {
                                        match ncmapi.client.song_list_detail(songlist.id).await {
                                            Ok(detail) => {
                                                sender
                                                    .send(Action::SetupMyPageSongs(
                                                        section,
                                                        take_preview(
                                                            detail.songs,
                                                            MY_PAGE_SONG_PREVIEW_LIMIT,
                                                        ),
                                                    ))
                                                    .await
                                                    .unwrap();
                                            }
                                            Err(err) => {
                                                fail_my_page_request(&sender, section, err)
                                            }
                                        }
                                    } else {
                                        sender
                                            .send(Action::SetupMyPageSongs(section, Vec::new()))
                                            .await
                                            .unwrap();
                                    }
                                }
                                Err(err) => fail_my_page_request(&sender, section, err),
                            }
                        }
                        MyPageSection::FavoriteAlbums => {
                            match ncmapi
                                .client
                                .album_sublist(0, MY_PAGE_COLLECTION_PREVIEW_LIMIT as u16)
                                .await
                            {
                                Ok(albums) => {
                                    sender
                                        .send(Action::SetupMyPageCollections(
                                            section,
                                            take_preview(
                                                albums,
                                                MY_PAGE_COLLECTION_PREVIEW_LIMIT,
                                            ),
                                        ))
                                        .await
                                        .unwrap();
                                }
                                Err(err) => fail_my_page_request(&sender, section, err),
                            }
                        }
                        MyPageSection::FavoriteSongLists => {
                            match ncmapi
                                .client
                                .user_song_list(
                                    window.get_uid(),
                                    0,
                                    (MY_PAGE_COLLECTION_PREVIEW_LIMIT + 1) as u16,
                                )
                                .await
                            {
                                Ok(songlists) => {
                                    sender
                                        .send(Action::SetupMyPageCollections(
                                            section,
                                            skip_liked_playlist(
                                                songlists,
                                                Some(MY_PAGE_COLLECTION_PREVIEW_LIMIT),
                                            ),
                                        ))
                                        .await
                                        .unwrap();
                                }
                                Err(err) => fail_my_page_request(&sender, section, err),
                            }
                        }
                    }
                });
            }
            Action::SetupMyPageSongs(section, songs) => {
                window.update_my_page_songs(section, songs);
            }
            Action::SetupMyPageCollections(section, items) => {
                window.update_my_page_collections(section, items);
            }
            Action::FailMyPageSection(section) => {
                window.fail_my_page_section(section);
            }
```

- [ ] **Step 8: 更新中文翻译**

在 `po/zh_CN.po` 增加：

```po
#: data/gtk/my-page.ui
msgid "No content"
msgstr "暂无内容"

#: data/gtk/my-page.ui
msgid "Failed to load"
msgstr "加载失败"

#: data/gtk/my-page.ui
msgid "Retry"
msgstr "重试"
```

`Daily Recommendation`、`Favorite Songs`、`Favorite Album`、`Favorite Song List` 和 `View More` 已存在，不重复添加。

- [ ] **Step 9: 格式化、测试与构建**

Run:

```bash
cargo fmt
cargo test preview_helpers -- --nocapture
cargo build
ninja -C _build
```

Expected:

- 预览助手测试 `2 passed; 0 failed`。
- Cargo 构建无错误、无新增 warning。
- Ninja 成功重新编译 gresource，模板 ID、回调名和 GObject 类型全部有效。

- [ ] **Step 10: Commit**

```bash
git add src/model.rs data/gtk/my-page.ui src/gui/my_page.rs src/window.rs src/application.rs po/zh_CN.po
git commit -m "feat: 将我的页重构为四分区内容预览"
```

---

### Task 4: 完整收藏歌单安全跳过首项

**Files:**
- Modify: `src/application.rs:1249-1264`

**Interfaces:**
- Consumes: Task 1 的 `skip_liked_playlist`
- Preserves: `ToMyPageSonglist` 的完整页导航与一次性请求行为

- [ ] **Step 1: 替换不安全切片**

将 `ToMyPageSonglist` 成功分支中的：

```rust
page.update_songlist(&sls[1..]);
```

替换为：

```rust
let songlists = skip_liked_playlist(sls, None);
page.update_songlist(&songlists);
```

- [ ] **Step 2: 运行边界测试和构建**

Run:

```bash
cargo test preview_helpers -- --nocapture
cargo build
```

Expected: 测试 `2 passed; 0 failed`，构建成功；空数组和仅含首项时均由测试证明不会越界。

- [ ] **Step 3: Commit**

```bash
git add src/application.rs
git commit -m "fix: 避免空收藏歌单列表切片越界"
```

---

### Task 5: 全量验证与 GUI 验收

**Files:**
- Verify: `src/application.rs`
- Verify: `src/model.rs`
- Verify: `src/window.rs`
- Verify: `src/gui/my_page.rs`
- Verify: `src/gui/songlist_row.rs`
- Verify: `data/gtk/my-page.ui`
- Verify: `po/zh_CN.po`

**Interfaces:**
- Consumes: Task 1–4 全部产出
- Produces: 可交付的构建和人工验收记录

- [ ] **Step 1: 静态与测试验证**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy
```

Expected: 单测全部通过；无新增 clippy warning。若仓库存在历史 warning，只记录并确认不来自本次改动。

- [ ] **Step 2: Meson 全量构建与数据校验**

Run:

```bash
ninja -C _build
meson test -C _build
```

Expected: 构建成功；desktop、metainfo、gschema 数据测试全部通过。

- [ ] **Step 3: 检查 IDE linter**

检查 Task 1–4 的全部改动文件。Expected: 无本次改动引入的错误或警告。

- [ ] **Step 4: 运行 GUI 冒烟**

Run:

```bash
RUST_LOG=debug make run
```

人工检查：

1. 登录后只显示四个分区，不再出现六个快捷图标和“推荐歌单”。
2. 分区分别最多显示 8、8、10、10 项。
3. 默认/宽窗口歌曲为两列；最小宽度窗口叠为单列，顺序保持 1–8。
4. 歌曲行显示歌曲名、歌手、时长、收藏操作；不显示专辑文字、专辑按钮和移除按钮。
5. 点击歌曲播放，切换歌曲时旧播放指示取消。
6. 点击专辑和歌单卡片进入正确详情。
7. 四个“更多”进入正确完整页，返回仍在“我的”页。
8. 单区失败时其他区继续加载；重试只刷新失败区。
9. 空收藏歌单和仅含“喜欢的音乐”的情况显示空状态且不崩溃。
10. 注销后保持未登录页，迟到请求不主动切回登录页。
11. 明暗主题均无 GTK critical 日志。

若当前环境无法登录或无图形显示，明确记录未执行的人工项，不得宣称 GUI 验收完成。

- [ ] **Step 5: 最终工作区检查**

Run:

```bash
git status --short
git diff --check
```

Expected: 仅有用户原有未跟踪文件；功能改动均已提交，diff 无空白错误。
