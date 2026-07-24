# 本地免安装构建并运行 netease-cloud-music-gtk4
#
# 用法:
#   make run     # Linux/macOS：构建并直接运行（无需 root，不写入系统目录）
#   make build   # Linux/macOS：只构建并安装到 _local/prefix
#   make clean   # 删除本地构建目录 _local
#   make dev     # Windows：MSVC 构建 → 同步/打包便携包（含 DLL）→ 启动
#                # Linux/macOS：同 make run
#
# Linux/macOS 原理: 使用独立的构建目录 _local，并将 meson 的 prefix 指向
# 项目内的 _local/prefix，使编译期写入 config.rs 的 PKGDATADIR / LOCALEDIR
# 都是本地绝对路径；meson install 到该本地前缀后，gresource、GSettings
# schema、gettext 翻译均可被找到。GSettings schema 通过
# GSETTINGS_SCHEMA_DIR 指向本地编译产物。
#
# Windows 原理: 调用 build-aux/windows/dev.ps1——先 meson/cargo 安装到
# _windows/install，再同步到已含 DLL 的 _windows/dist 便携包（首次或缺包时
# 完整 package）；从便携包目录启动 exe，避免裸 install\bin 缺 DLL。

BUILDDIR  ?= _local
PREFIX    := $(CURDIR)/$(BUILDDIR)/prefix
APP       := netease-cloud-music-gtk4
SCHEMADIR := $(PREFIX)/share/glib-2.0/schemas
# 本地运行默认 debug 构建（编译更快），可用 make BUILDTYPE=release 覆盖
BUILDTYPE ?= debug

.PHONY: run build clean dev

run: build
	GSETTINGS_SCHEMA_DIR="$(SCHEMADIR)" "$(PREFIX)/bin/$(APP)"

build:
	@if [ -f "$(BUILDDIR)/build.ninja" ]; then \
		meson setup --reconfigure "$(BUILDDIR)" -Dprefix="$(PREFIX)" -Dbuildtype="$(BUILDTYPE)"; \
	else \
		meson setup "$(BUILDDIR)" -Dprefix="$(PREFIX)" -Dbuildtype="$(BUILDTYPE)"; \
	fi
	meson compile -C "$(BUILDDIR)"
	meson install -C "$(BUILDDIR)"

clean:
	rm -rf "$(BUILDDIR)"

ifeq ($(OS),Windows_NT)
dev:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(CURDIR)/build-aux/windows/dev.ps1" -BuildType "$(BUILDTYPE)"
else
dev: run
endif
