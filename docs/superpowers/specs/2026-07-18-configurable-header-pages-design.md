# 可配置 Header 页面设计（显示开关 + 顺序调整）

日期：2026-07-18
分支：`feat/configurable-header-pages`（worktree：`worktrees/configurable-header-pages`）

## 背景与目标

主窗口 header 的 `AdwViewSwitcher` 固定展示「发现（discover）/ 榜单（toplist）/ 我的（my）」三个页面入口。部分用户不需要「发现」和「榜单」，希望：

1. 在首选项中控制这三个入口是否显示，其中「我的」始终显示（无开关）；
2. 三个入口的排列顺序可以调整。

改动即时生效，无需重启应用。

## 现状分析

- `data/gtk/window.ui`：`AdwViewStack id="stack"` 内含四个 `AdwViewStackPage`：`discover`、`toplist`、`my`（内嵌 `my_stack`）、`search`（占位页，`visible=false`）。header 的 `AdwViewSwitcher id="switcher_title"`（policy=wide）与底部的 `AdwViewSwitcherBar` 均绑定该 stack。
- `AdwViewStackPage.visible=false` 即可让页面从两个切换器中消失（search 页已是此用法）。
- libadwaita 的 `AdwViewStack` 无 reorder/insert API（已核对本机 `/usr/include/libadwaita-1/adw-view-stack.h`，仅 `add*` 追加与 `remove`），重排需 remove 后按序重新 add。
- `src/gui/preferences.rs` + `data/gtk/preferences.ui`：`AdwPreferencesDialog`，控件在 `constructed` 中通过 `gio::Settings::bind` 绑定 GSettings。
- GSettings schema：`data/com.gitee.gmg137.NeteaseCloudMusicGtk4.gschema.xml`。
- `src/window.rs:229` 已持有 `Settings` 实例（`imp.settings`）。
- discover/toplist 的数据加载发生在 `window.rs` 的 `init_page_data()`（`discover.init_page()` 与 `Action::GetToplist`）。

## GSettings 设计

在 schema 中新增三个键：

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `pages-order` | `as` | `['discover', 'toplist', 'my']` | 三页排列顺序，始终包含全部三个名字（含隐藏页） |
| `show-discover` | `b` | `true` | 是否显示「发现」 |
| `show-toplist` | `b` | `true` | 是否显示「榜单」 |

「我的」无开关，恒为可见。

## 主窗口改动（`src/window.rs`）

新增方法 `apply_pages_config()`：

1. 读取 `pages-order` 并清洗：去重、丢弃未知名、按默认顺序补齐缺失项，保证结果是 `discover/toplist/my` 三个有效名的某种排列。
2. 记录当前 `stack.visible_child_name()`。
3. 设置可见性：`discover`/`toplist` 按 `show-*` 设置各自 `AdwViewStackPage.visible`；`my` 恒 `true`；`search` 保持 `false`。
4. 重排：将 `discover/toplist/my/search` 四个 child 全部从 stack `remove`，再按「清洗后的三页顺序 + search」用 `add_titled_with_icon`（search 用 `add`）依次追加。name/title/icon-name 在 remove 前从各 page 读出，add 时原样写回。这些页面均未使用 badge/needs-attention 等其他 page 属性，无信息丢失。
5. 恢复可见页：若第 2 步记录的页面仍可见，则 `set_visible_child_name` 恢复；否则切换到顺序中第一个可见页。启动时因此落在「排在最前的可见页」。

调用时机与联动：

- 窗口显示前调用一次：在 `init_page_data()` 开头调用，保证在任何页面数据填充与窗口展示前完成重排，避免可见闪烁。
- 复用 `imp.settings`，对 `pages-order`、`show-discover`、`show-toplist` 三个键分别 `connect_changed`，回调中调用 `apply_pages_config()`，实现首选项改动即时生效。

导航历史代码（`window.rs` 中按 `discover/toplist/my` 名字判断的逻辑）不受影响，页面名字不变。

数据加载行为不变：隐藏的页面仍在 `init_page_data()` 中初始化数据（discover 轮播/推荐、toplist 列表照常请求），不做额外裁剪。

## 首选项改动（`data/gtk/preferences.ui` + `src/gui/preferences.rs`）

`preferences.ui`：

- 在 "General" 组之后新增一个 `AdwPreferencesGroup id="pages_group"`，标题 "Header Pages"（可翻译），组内容留空，由代码填充。

`preferences.rs`：

- `imp` 增加 `pages_group: TemplateChild<adw::PreferencesGroup>`。
- `constructed` 中调用 `rebuild_page_rows()`：
  - 读取（并清洗，同窗口逻辑）`pages-order`，为每个页面名创建一行 `AdwActionRow`：
    - 标题：Discover / Toplist / My，使用与 window.ui 相同的 msgid（gettext 自动复用现有翻译）。
    - 后缀：上移按钮（`go-up-symbolic`）、下移按钮（`go-down-symbolic`）。首行禁用上移、末行禁用下移。
    - `discover`/`toplist` 行额外带 `GtkSwitch`，`settings.bind("show-*", switch, "active")`；`my` 行无开关。
  - 上/下移按钮回调：交换 `pages-order` 中对应位置并 `set_strv` 写回，随后 `rebuild_page_rows()` 重建行（重建前先移除组内旧行；行上控件销毁时 GSettings 绑定自动解除）。
  - 窗口端通过 settings 变更信号同步刷新，无需首选项与窗口直接通信。

## 国际化

- 新用户可见字符串（"Header Pages" 等）用 `translatable="yes"` / `gettext(...)` 包裹。
- `po/zh_CN.po` 增加对应中文翻译条目。
- `po/POTFILES` 已包含 `data/gtk/preferences.ui`，无需改动。

## 工程化与验证

- 开发在 worktree `worktrees/configurable-header-pages`（分支 `feat/configurable-header-pages`）内进行；主仓库 `.gitignore` 已加入 `/worktrees`。
- 验证：
  - `cargo build` 编译通过（先 `meson setup _build` 生成 `src/config.rs`）。
  - `glib-compile-schemas --strict --dry-run` 校验 schema 变更。
  - 手工运行验证：设置 `GSETTINGS_SCHEMA_DIR` 指向编译出的 schema 目录后运行，检查开关与排序即时生效、重启后保持、非法 `pages-order` 兜底正常。

## 明确不做（YAGNI）

- 隐藏页面时跳过其数据加载（保持现有加载行为）。
- 拖拽排序（采用上/下移按钮）。
- 「我的」页的显示开关（恒可见）。
- 记住上次退出时的可见页（启动始终落在顺序中第一个可见页）。
