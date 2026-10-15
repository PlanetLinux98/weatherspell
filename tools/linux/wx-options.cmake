# wxWidgets options for the Linux build, read through the cmake crate's
# CMAKE_TOOLCHAIN_FILE (set in .cargo/config.toml): wxdragon-sys gives no
# other way in. Each keeps the app asking only for libraries every GTK
# desktop has, under names that hold across distros
# (tools/linux/check-libraries.sh checks).
#
# JPEG and TIFF off: the app shows only SVG and PNG, and the system's
# libjpeg and libtiff differ by distro (libjpeg.so.8 on Ubuntu, .62 on
# Fedora and Arch; libtiff.so.5 on 22.04).
set(wxUSE_LIBJPEG OFF CACHE STRING "" FORCE)
set(wxUSE_LIBTIFF OFF CACHE STRING "" FORCE)

# OpenGL off: the app draws nothing with it, and a build on Ubuntu 22.04
# kept wx's GL canvas, asking for libEGL, which a desktop need not have
# (Debian 12 and Ubuntu 22.04 with only GTK 3 lack it). wxdragon-sys still
# names wx's GL library to the linker, so an empty one stands in, somewhere
# under the build folder it searches.
set(wxUSE_OPENGL OFF CACHE BOOL "" FORCE)
if(NOT EXISTS "${CMAKE_BINARY_DIR}/no-gl/libwx_gtk3u_gl-3.3.a")
    file(WRITE "${CMAKE_BINARY_DIR}/no-gl/libwx_gtk3u_gl-3.3.a" "!<arch>\n")
endif()
