# The RPM for Copr (D-144). Copr builds it with internet access: cargo fetches the crates
# from crates.io at the versions in Cargo.lock.

%global app_id io.github.kotoanidev.RustBotCreator
# The release build has no debug info to split out.
%global debug_package %{nil}

Name:           rustbot-creator
Version:        0.1.0
Release:        1%{?dist}
Summary:        Make Discord bots in Rust without writing code
License:        MIT OR Apache-2.0
URL:            https://github.com/kotoanidev/rustbot-creator
Source0:        %{name}-%{version}.tar.gz
# Release 1 supports only the tested architecture (D-146).
ExclusiveArch:  x86_64

BuildRequires:  rust >= 1.92
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  pkgconf-pkg-config
BuildRequires:  fontconfig-devel
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  libxkbcommon-x11-devel
BuildRequires:  libxcb-devel
BuildRequires:  libX11-devel
BuildRequires:  openssl-devel
BuildRequires:  libzstd-devel
BuildRequires:  alsa-lib-devel
BuildRequires:  python3
BuildRequires:  desktop-file-utils
BuildRequires:  appstream

# Opened at run time by the graphics stack, so rpm cannot find them by itself.
Requires:       libvulkan.so.1()(64bit)
Requires:       libEGL.so.1()(64bit)
Requires:       libwayland-client.so.0()(64bit)
Requires:       libwayland-egl.so.1()(64bit)
Requires:       libfontconfig.so.1()(64bit)
Recommends:     mesa-vulkan-drivers
# Bots are built with the system's C compiler (D-039, D-148).
Recommends:     gcc

%description
RustBot Creator builds Discord bots on the djs-rs library. Each slash command
and event is a graph of nodes, and the app turns it into readable Rust, builds
it and runs it, exports it, or hosts it on this PC. It is pre-alpha.

%prep
%autosetup

%build
export CARGO_HOME=%{_builddir}/cargo-home
cargo build --release --locked -p rustbot-creator
# cargo metadata reads every crate in Cargo.lock, so it fetches the dev-dependencies too.
cargo metadata --format-version 1 --locked \
    --filter-platform %{_arch}-unknown-linux-gnu \
    | python3 -I packaging/third-party-licenses.py > THIRD-PARTY-LICENSES.txt

%install
install -Dpm 0755 target/release/rustbot-creator %{buildroot}%{_bindir}/rustbot-creator
install -Dpm 0644 packaging/linux/%{app_id}.desktop \
    %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dpm 0644 packaging/linux/%{app_id}.metainfo.xml \
    %{buildroot}%{_metainfodir}/%{app_id}.metainfo.xml
install -Dpm 0644 packaging/linux/%{app_id}.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop
appstreamcli validate --no-net %{buildroot}%{_metainfodir}/%{app_id}.metainfo.xml
# The hosted bot's launcher (D-138) needs no display: a wrong ID stops it with 78.
%{buildroot}%{_bindir}/rustbot-creator --host not-a-uuid bot || test $? -eq 78

%files
%license LICENSE-MIT LICENSE-APACHE THIRD-PARTY-LICENSES.txt
%doc README.md
%{_bindir}/rustbot-creator
%{_datadir}/applications/%{app_id}.desktop
%{_metainfodir}/%{app_id}.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%changelog
* Tue Nov 24 2026 kotoanidev <kotoanidev@users.noreply.github.com> - 0.1.0-1
- The first pre-alpha.
