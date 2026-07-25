#!/usr/bash
# netease-cloud-music-gtk4: replaces gvsbuild's patches/ffmpeg/build/build.sh
# (applied by build-aux/windows/bootstrap.ps1). Differences from upstream:
#   1. sed-patch configure so the MSVC probe matches localized cl banners.
#   2. Enable the audio decoders gst-libav needs for NetEase streams
#      (mp3/flac/aac); gvsbuild only enables video decoders + mp2/wma.

prefix="$1"
gtk_dir="$2"
build_type="$3"
enable_gpl="$4"

# VS Build Tools without the English language pack prints a localized cl
# banner (VSLANG=1033 has no effect), so configure's `grep ^Microsoft` probe
# fails, the compiler stays "unknown", and the object flag falls back to a
# `-o` form that MSVC 19.44+ ignores (LNK1136/LNK1181). Match the banner text
# anywhere in the line instead of anchoring at the start.
sed -i 's|grep -q \^Microsoft|grep -q "Microsoft (R)"|g; s|grep \^Microsoft|grep "Microsoft (R)"|g' configure

declare -a configure_cmd
declare -i idx=0

configure_cmd[idx++]="./configure"
configure_cmd[idx++]="--toolchain=msvc"
configure_cmd[idx++]="--prefix=\"$prefix\""
configure_cmd[idx++]="--enable-shared"
configure_cmd[idx++]="--disable-everything"
configure_cmd[idx++]="--enable-swscale"
configure_cmd[idx++]="--enable-avcodec"
configure_cmd[idx++]="--enable-avfilter"
configure_cmd[idx++]="--enable-avformat"
configure_cmd[idx++]="--enable-hwaccel=av1_dxva2"
configure_cmd[idx++]="--enable-hwaccel=h264_dxva2"
configure_cmd[idx++]="--enable-hwaccel=hevc_dxva2"
configure_cmd[idx++]="--enable-dxva2"
configure_cmd[idx++]="--enable-decoder=h264"
configure_cmd[idx++]="--enable-decoder=hevc"
configure_cmd[idx++]="--enable-decoder=libdav1d"
configure_cmd[idx++]="--enable-decoder=mpeg1video"
configure_cmd[idx++]="--enable-encoder=mpeg1video"
configure_cmd[idx++]="--enable-hwaccel=av1_d3d11va"
configure_cmd[idx++]="--enable-hwaccel=av1_d3d11va2"
configure_cmd[idx++]="--enable-hwaccel=h264_d3d11va"
configure_cmd[idx++]="--enable-hwaccel=h264_d3d11va2"
configure_cmd[idx++]="--enable-hwaccel=hevc_d3d11va"
configure_cmd[idx++]="--enable-hwaccel=hevc_d3d11va2"
configure_cmd[idx++]="--enable-libdav1d"
configure_cmd[idx++]="--enable-d3d11va"
configure_cmd[idx++]="--enable-nvdec"
configure_cmd[idx++]="--enable-hwaccel=av1_nvdec"
configure_cmd[idx++]="--enable-hwaccel=h264_nvdec"
configure_cmd[idx++]="--enable-hwaccel=hevc_nvdec"
configure_cmd[idx++]="--disable-programs"
configure_cmd[idx++]="--disable-avdevice"
configure_cmd[idx++]="--disable-swresample"

# Enable audio decoder which aren't available in native gst plugins, expected to be used via gst-libav
configure_cmd[idx++]="--enable-decoder=mp2float"
configure_cmd[idx++]="--enable-decoder=wmav2"
configure_cmd[idx++]="--enable-decoder=wmapro"
# NetEase Cloud Music serves mp3 (all standard rates) and flac (lossless and
# above); aac covers occasional m4a. Pair them with their parsers so gst-libav
# can register avdec_mp3/avdec_flac/avdec_aac.
configure_cmd[idx++]="--enable-decoder=mp3float"
configure_cmd[idx++]="--enable-decoder=flac"
configure_cmd[idx++]="--enable-decoder=aac"
configure_cmd[idx++]="--enable-parser=mpegaudio"
configure_cmd[idx++]="--enable-parser=flac"
configure_cmd[idx++]="--enable-parser=aac"

if [ "$build_type" = "debug" ]; then
    configure_cmd[idx++]="--enable-debug"
    # FIXME: the -Od and -Zi instructions are overriden in the compilation command
    configure_cmd[idx++]="--extra-cflags=-MDd -Od -Zi"
else
    configure_cmd[idx++]="--extra-cflags=-MD"
fi

if [ "$build_type" = "debug-optimized" ]; then
    configure_cmd[idx++]="--extra-ldflags=-DEBUG:FULL"
    configure_cmd[idx++]="--extra-cflags=-Zi"
fi

if [ "$enable_gpl" = "enable_gpl" ]; then
    configure_cmd[idx++]="--enable-libx264"
    configure_cmd[idx++]="--enable-gpl"
    configure_cmd[idx++]="--enable-encoder=libx264"
fi

export PKG_CONFIG_PATH=$gtk_dir/lib/pkgconfig:$PKG_CONFIG_PATH

"${configure_cmd[@]}"

make
make install
