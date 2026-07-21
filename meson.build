project('netease-cloud-music-gtk4', 'rust',
          version: '2.5.3',
    meson_version: '>= 0.59.0',
  default_options: [ 'warning_level=2',
                     'werror=false',
                     'buildtype=release'
                   ],
)

i18n = import('i18n')

gnome = import('gnome')
host_system = host_machine.system()

dependency('openssl', version: '>= 1.0')
if host_system == 'linux'
  dependency('dbus-1')
endif

dependency('glib-2.0', version: '>= 2.66')
dependency('gio-2.0', version: '>= 2.66')
dependency('gdk-pixbuf-2.0')
dependency('gtk4', version: '>= 4.10')
dependency('libadwaita-1', version: '>=1.5.0')

dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-base-1.0', version: '>= 1.16')
dependency('gstreamer-audio-1.0', version: '>= 1.16')
dependency('gstreamer-play-1.0', version: '>= 1.16')
# Plugin package .pc files are mainly useful on Linux distro packaging.
# Windows portable builds ship plugins via the MSVC prefix instead.
if host_system == 'linux'
  dependency('gstreamer-plugins-base-1.0', version: '>= 1.16')
  dependency('gstreamer-plugins-bad-1.0', version: '>= 1.16')
  dependency('gstreamer-bad-audio-1.0', version: '>= 1.16')
endif

cargo_sources = files(
  'Cargo.toml',
)

subdir('data')
subdir('src')
subdir('po')

if host_system == 'linux'
  gnome.post_install(
    glib_compile_schemas: true,
    gtk_update_icon_cache: true,
    update_desktop_database: true,
  )
endif
