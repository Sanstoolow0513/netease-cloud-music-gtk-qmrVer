# 本地免安装构建并运行 netease-cloud-music-gtk4
#
# 用法:
#   make run     # 构建并直接运行（无需 root，不写入系统目录）
#   make build   # 只构建并安装到 _local/prefix
#   make clean   # 删除本地构建目录 _local
#
# 原理: 使用独立的构建目录 _local，并将 meson 的 prefix 指向项目内的
# _local/prefix，使编译期写入 config.rs 的 PKGDATADIR / LOCALEDIR 都是
# 本地绝对路径；meson install 到该本地前缀后，gresource、GSettings schema、
# gettext 翻译均可被找到，二进制即可直接运行。GSettings schema 通过
# GSETTINGS_SCHEMA_DIR 环境变量指向本地编译产物。

BUILDDIR  ?= _local
PREFIX    := $(CURDIR)/$(BUILDDIR)/prefix
APP       := netease-cloud-music-gtk4
SCHEMADIR := $(PREFIX)/share/glib-2.0/schemas
# 本地运行默认 debug 构建（编译更快），可用 make BUILDTYPE=release 覆盖
BUILDTYPE ?= debug

.PHONY: run build clean

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
