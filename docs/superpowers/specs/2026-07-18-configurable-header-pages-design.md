# 可配置 Header 页面设计（显示开关 + 顺序调整）

日期：2026-07-18（2026-07-19 修订：改为重启生效）
分支：`feat/configurable-header-pages`（worktree：`worktrees/configurable-header-pages`）

## 背景与目标

主窗口 header 的 `AdwViewSwitcher` 固定展示「发现（discover）/ 榜单（toplist）/ 我的（my）」三个页面入口。部分用户不需要「发现」和「榜单」，希望：

1. 在首选项中控制这三个入口是否显示，其中「我的」始终显示（无开关）；
2. 三个入口的排列顺序可以调整。

配置写入 GSettings 后**重启应用生效**（不做运行时热更新）。

## 现状分析

- `data/gtk/window.ui`：`AdwViewStack id="stack"` 内含四个 `AdwViewStackPage`：`discover`、`toplist`、`my`（内嵌 `my_stack`）、`search`（占位页，`visible=false`）。header 的 `AdwViewSwitcher id="switcher_title"`（policy=wide）与底部的 `AdwViewSwitcherBar` 均绑定该 stack。
- `AdwViewStackPage.visible=false` 即可让页面从两个切换器中消失（search 页已是此用法）。
- libadwaita 的 `AdwViewStack` 无 reorder/insert API（已核对本机 `/usr/include/libadwaita-1/adw-view-stack.h`，仅 `add*` 追加与 `remove`），重排需 remove 后按序重新 add。
- `src/gui/preferences.rs` + `data/gtk/preferences.ui`：`AdwPreferencesDialog`，控件在 `constructed` 中通过 `gio::Settings::bind` 绑定 GSettings。
- GSettings schema：`data/com.gitee.gmg137.NeteaseCloudMusicGtk4.gschema.xml`。
- `src/window.rs` 已持有 `Settings` 实例（`imp.settings`）。
- discover/toplist 的数据加载发生在 `window.rs` 的 `init_page_data()`（`discover.init_page()` 与 `Action::GetToplist`）。

## GSettings 设计

在 schema 中新增三个键：

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `pages-order` | `as` | `['discover', 'toplist', 'my']` | 三页排列顺序，始终包含全部三个名字（含隐藏页） |
| `show-discover` | `b` | `true` | 是否显示「发现」 |
| `show-toplist` | `b` | `true` | 是否显示「榜单」 |

「我的」无开关，恒为可见。

共享清洗逻辑见 `src/utils.rs` 的 `sanitize_pages_order()`：去重、丢弃未知名、按默认顺序补齐缺失项。

## 主窗口改动（`src/window.rs`）

新增方法 `apply_pages_config()`：

1. 读取并清洗 `pages-order`，读取 `show-discover` / `show-toplist`。
2. 设置可见性：`discover`/`toplist` 按 `show-*` 设置各自 `AdwViewStackPage.visible`；`my` 恒 `true`；`search` 保持 `false`。
3. 重排：将 `discover/toplist/my/search` 四个 child 全部从 stack `remove`，再按「清洗后的三页顺序 + search」用 `add_titled_with_icon`（search 用 `add`）依次追加。name/title/icon-name 在 remove 前从各 page 读出，add 时原样写回。
4. `set_visible_child_name` 设为顺序中第一个可见页；同步将 `stack_child` 导航历史重置为该起始页（覆盖 `constructed` 中硬编码的 `"discover"`）。

调用时机：仅在 `init_page_data()` 开头调用一次。不做 `connect_changed` 热更新。

导航历史代码（按 `discover/toplist/my` 名字判断的逻辑）不受影响，页面名字不变。

数据加载行为不变：隐藏的页面仍在 `init_page_data()` 中初始化数据，不做额外裁剪。

## 首选项改动（`data/gtk/preferences.ui` + `src/gui/preferences.rs`）

`preferences.ui`：

- 在 "General" 组之后新增 `AdwPreferencesGroup id="pages_group"`，标题 "Header Pages"，subtitle "Takes effect after restart"（均可翻译），组内容留空，由代码填充。

`preferences.rs`：

- `imp` 增加 `pages_group: TemplateChild<adw::PreferencesGroup>`。
- `constructed` 中调用 `rebuild_page_rows()`：
  - 读取（并清洗）`pages-order`，为每个页面名创建一行 `AdwActionRow`：
    - 标题：Discover / Toplist / My，使用与 window.ui 相同的 msgid。
    - 后缀：上移按钮（`go-up-symbolic`）、下移按钮（`go-down-symbolic`）。首行禁用上移、末行禁用下移。
    - `discover`/`toplist` 行额外带 `GtkSwitch`，`settings.bind("show-*", switch, "active")`；`my` 行无开关。
  - 上/下移按钮回调：交换 `pages-order` 中对应位置并 `set_strv` 写回，随后 `rebuild_page_rows()` 重建行。

## 国际化

- 新用户可见字符串用 `translatable="yes"` / `gettext(...)` 包裹。
- `po/zh_CN.po` 增加对应中文翻译条目。
- `po/POTFILES` 已包含 `data/gtk/preferences.ui`，无需改动。

## 工程化与验证

- 开发在 worktree `worktrees/configurable-header-pages`（分支 `feat/configurable-header-pages`）内进行；主仓库 `.gitignore` 已加入 `/worktrees`。
- 验证：
  - `cargo build` 编译通过（先 `meson setup _build` 生成 `src/config.rs`）。
  - `glib-compile-schemas --strict --dry-run` 校验 schema 变更。
  - 手工：改设置 → 重启后开关/顺序正确；非法 `pages-order` 兜底正常。

## 明确不做（YAGNI）

- 运行时热更新（需重启生效）。
- 隐藏页面时跳过其数据加载（保持现有加载行为）。
- 拖拽排序（采用上/下移按钮）。
- 「我的」页的显示开关（恒可见）。
- 记住上次退出时的可见页（启动始终落在顺序中第一个可见页）。
