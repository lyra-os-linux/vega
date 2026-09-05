# Empacotamento Linux do vega-gtk. Versionamento via --define version
# (ver Version abaixo); ver packaging/obs/vega-gtk.spec para a variante
# consumida pelo OBS via tar_scm.
%{!?version: %define version 0.0.0}
Name:           vega-gtk
Version:        %{version}
Release:        1%{?dist}
Summary:        Centro de controle para Linux
License:        GPL-3.0-only
URL:            https://github.com/lyra-os-linux/vega
Source0:        vega-src.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  gettext-tools
Requires:       vegad
Requires:       secret-tool
Requires:       python3-gobject
Requires:       typelib-1_0-Atspi-2_0
Provides:       vega = %{version}-%{release}
Obsoletes:      vega < %{version}-%{release}
# vega-gtk se chamava lyra-vega-gtk antes do projeto deixar de ser
# exclusivo do LyraOS — Provides/Obsoletes garante upgrade sem conflito
# pra quem já tinha o pacote com o nome antigo instalado.
Provides:       lyra-vega-gtk = %{version}-%{release}
Obsoletes:      lyra-vega-gtk < %{version}-%{release}

Recommends:     flatpak
Recommends:     restic

%description
Interface nativa do Vega, construída com Rust, GTK4 e libadwaita.

%prep
%setup -q -c -n vega-src

%build
cd vega-gtk
VEGA_VERSION=%{version} cargo build --release --locked

%install
install -Dm755 target/release/vega-gtk \
  %{buildroot}%{_bindir}/vega-gtk
install -Dm755 target/release/vega-update-notifier \
  %{buildroot}%{_bindir}/vega-update-notifier
ln -s vega-gtk %{buildroot}%{_bindir}/lyra-vega-gtk

install -Dm644 packaging/vega/vega.desktop \
  %{buildroot}%{_datadir}/applications/vega.desktop
install -Dm644 packaging/vega/org.lyraos.Vega.UpdateNotifier.desktop \
  %{buildroot}%{_datadir}/applications/org.lyraos.Vega.UpdateNotifier.desktop
install -Dm644 packaging/vega/vega-update-notifier.desktop \
  %{buildroot}%{_sysconfdir}/xdg/autostart/vega-update-notifier.desktop
install -Dm644 packaging/vega/vega.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/vega.svg
install -Dm644 packaging/vega/icons/lyra-updates-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/lyra-updates-symbolic.svg
for locale in en_US pt_BR es_ES; do
  install -Dm644 "vega-gtk/po/locale/${locale}/LC_MESSAGES/vega-gtk.mo" \
    "%{buildroot}%{_datadir}/locale/${locale}/LC_MESSAGES/vega-gtk.mo"
done

%files
%{_bindir}/vega-gtk
%{_bindir}/vega-update-notifier
%{_bindir}/lyra-vega-gtk
%{_datadir}/applications/vega.desktop
%{_datadir}/applications/org.lyraos.Vega.UpdateNotifier.desktop
%{_sysconfdir}/xdg/autostart/vega-update-notifier.desktop
%{_datadir}/icons/hicolor/scalable/apps/vega.svg
%{_datadir}/icons/hicolor/symbolic/apps/lyra-updates-symbolic.svg
%lang(en) %{_datadir}/locale/en_US/LC_MESSAGES/vega-gtk*.mo
%lang(pt_BR) %{_datadir}/locale/pt_BR/LC_MESSAGES/vega-gtk*.mo
%lang(es) %{_datadir}/locale/es_ES/LC_MESSAGES/vega-gtk*.mo

%changelog
