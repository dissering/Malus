%global app_id io.github.dissering.Tiltframe
%global rosemary_archive Malus-Rosemary-0.5.0-quadra.3-malus.1-x86_64.tar.gz
%global debug_package %{nil}

Name:           malus
Version:        0.1.1
Release:        1%{?dist}
Summary:        Roblox Player and Studio launcher for Linux through Rosemary
License:        GPL-3.0-only AND OFL-1.1
URL:            https://github.com/dissering/Malus
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz
Source1:        %{url}/releases/download/v%{version}/%{rosemary_archive}
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
The Rosemary runtime is bundled. Roblox is downloaded when needed.

%prep
%autosetup -n Malus-%{version}

%build
cargo build --release --frozen

%install
printf '%%s  %%s\n' "$(cat packaging/rosemary.sha256)" '%{SOURCE1}' | sha256sum --check -
install -Dm755 target/release/malus %{buildroot}%{_bindir}/malus
install -Dm644 %{SOURCE1} %{buildroot}%{_datadir}/malus/rosemary.tar.gz
install -Dm644 data/%{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dm644 data/%{app_id}.xml %{buildroot}%{_datadir}/mime/packages/%{app_id}.xml
for icon in data/icons/hicolor/*/apps/%{app_id}.png; do
    install -Dm644 "$icon" %{buildroot}%{_datadir}/"${icon#data/}"
done

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop
%{buildroot}%{_bindir}/malus --version

%files
%license LICENSE assets/fonts/*-OFL.txt
%{_bindir}/malus
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/*/apps/%{app_id}.png
%{_datadir}/mime/packages/%{app_id}.xml
%dir %{_datadir}/malus
%{_datadir}/malus/rosemary.tar.gz
