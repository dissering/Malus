%global revision ed1ef33f9ebe66719a8e7cf7ca16cfefb1fbaa83
%global app_id io.github.dissering.Tiltframe
%global debug_package %{nil}

Name:           malus
Version:        0.1.0
Release:        1%{?dist}
Summary:        Roblox Player and Studio launcher for Linux through Rosemary
License:        GPL-3.0-only AND ISC AND OFL-1.1
URL:            https://github.com/dissering/Malus
Source0:        %{url}/archive/%{revision}.tar.gz#/%{name}-%{version}.tar.gz
ExclusiveArch:  x86_64

BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  desktop-file-utils
BuildRequires:  gcc
BuildRequires:  glib2-devel
BuildRequires:  gtk4-devel >= 4.18
BuildRequires:  libadwaita-devel >= 1.7
BuildRequires:  lld
BuildRequires:  pango-devel >= 1.56
BuildRequires:  pkgconf-pkg-config

Requires:       gtk4 >= 4.18
Requires:       hicolor-icon-theme
Requires:       libadwaita >= 1.7
Requires:       pango >= 1.56
Requires:       shared-mime-info

%description
Malus launches Roblox Player and Studio on Linux through Rosemary.
Rosemary and the Roblox client are downloaded when they are needed.

%prep
%autosetup -n Malus-%{revision}

%build
cargo build --release --frozen

%install
install -Dm755 target/release/malus %{buildroot}%{_bindir}/malus
install -Dm644 data/%{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dm644 data/%{app_id}.xml %{buildroot}%{_datadir}/mime/packages/%{app_id}.xml
for icon in data/icons/hicolor/*/apps/%{app_id}.png; do
    install -Dm644 "$icon" %{buildroot}%{_datadir}/"${icon#data/}"
done
install -Dm644 assets/lucide/LICENSE Lucide-LICENSE

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop
%{buildroot}%{_bindir}/malus --version

%files
%license LICENSE Lucide-LICENSE assets/fonts/*-OFL.txt
%{_bindir}/malus
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/*/apps/%{app_id}.png
%{_datadir}/mime/packages/%{app_id}.xml
