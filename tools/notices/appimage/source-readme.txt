The source of the AppImage runtime in Weatherspell-x86_64.AppImage

Weatherspell's Linux AppImage begins with AppImage's type2 runtime, a
small program that opens the AppImage and starts the app inside. The
runtime links libfuse, which is under the GNU Lesser General Public
Licence, version 2.1. This archive holds:

- type2-runtime-COMMIT/: the runtime's source at the release Weatherspell
  uses, with the scripts that build it and the one change it makes to
  libfuse (patches/libfuse/mount.c.diff).
- fuse-3.15.0.tar.xz: libfuse 3.15.0's source, as the runtime's build
  downloads it.

To build the runtime again, with libfuse changed or not, follow
BUILD.md in type2-runtime-COMMIT/. The runtime's licence and the
licences of everything it links are in Weatherspell's
THIRD-PARTY-NOTICES.md. Weatherspell's own source is at
https://github.com/PlanetLinux98/weatherspell.
