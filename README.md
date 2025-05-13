# flare.rs
A work-in-progress Rust based port of the classic Descent 3 engine


## Getting started

## Building with the "with_ffmpeg" feature (use ffmpeg)
```
cargo install cargo-vcpkg

sudo dnf install jack-audio-connection-kit-devel openal-soft-devel libcdio-paranoia-devel libcdio-devel alsa-lib-devel pulseaudio-libs-devel libv4l-devel libbs2b-devel lilv-devel rubberband-devel fftw-devel libsamplerate-devel libmysofa-devel fribidi-devel libplacebo-devel tesseract-devel libass-devel vid.stab-devel zimg-devel libmodplug-devel vapoursynth-devel libbluray-devel gnutls-devel srt-devel libssh-devel samba-devel zeromq-devel libvpx-devel opencore-amr-devel librsvg2-devel gdk-pixbuf2-devel zvbi-devel snappy-devel codec2-devel gsm-devel ilbc-devel lame-devel openjpeg2-devel opus-devel rav1e-devel speex-devel svt-av1-devel libogg-devel twolame-devel vo-amrwbenc-devel libvorbis-devel x264-devel x265-devel xvidcore-devel openh264-devel soxr-devel libva-devel libvdpau-devel libgcrypt-devel libgpg-error-devel ibopenmpt-devel libchromaprint-devel libsmbclient-devel libdav1d-devel libaom-devel libvma-devel libtheora-devel opencl-header libvpl-devel libgcrypt-devel

sudo dnf install nasm

cargo vcpkg --verbose build

cargo build --features with_ffmpeg
```
