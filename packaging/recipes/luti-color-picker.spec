%global debug_package %{nil}

Name: luti-color-picker
Version: %{luti_version}
Release: 1%{?dist}
Summary: Screen color picker with a magnifier and visual history
License: MIT
URL: https://github.com/eulucastiagolt/luti-color-picker
Source0: payload.tar.gz
Requires: gtk3 >= 3.24

%description
Capture screen colors, view your recent captures, and copy colors in eleven formats.

%prep
%setup -q -c -T
tar -xzf %{SOURCE0}

%install
mkdir -p %{buildroot}
cp -a usr %{buildroot}/

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%{_bindir}/luti-color-picker
%{_datadir}/applications/*.desktop
%{_datadir}/icons/hicolor/256x256/apps/*.png
%{_datadir}/metainfo/*.xml
