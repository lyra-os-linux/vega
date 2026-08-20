# Empacotamento Linux. Ver vegad.spec neste
# mesmo diretório para as notas gerais (versionamento via --define version,
# status do empacotamento).
%{!?version: %define version 0.0.0}
Name:           vega-gtk
Version:        %{version}
Release:        1%{?dist}
Summary:        Centro de controle para Linux
License:        GPL-3.0-only
URL:            https://github.com/britors/Vega
Source0:        vega-src.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  gettext-tools
Requires:       vegad
Requires:       secret-tool
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
install -Dm644 packaging/vega/vega-update-notifier.desktop \
  %{buildroot}%{_sysconfdir}/xdg/autostart/vega-update-notifier.desktop
install -Dm644 packaging/vega/vega.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/vega.svg
install -d %{buildroot}%{_datadir}/gnome-shell/extensions/updates-indicator@lyraos.org
install -m644 packaging/vega/updates-indicator@lyraos.org/{extension.js,metadata.json,stylesheet.css} \
  %{buildroot}%{_datadir}/gnome-shell/extensions/updates-indicator@lyraos.org/
for locale in en_US pt_BR es_ES; do
  install -Dm644 "vega-gtk/po/locale/${locale}/LC_MESSAGES/vega-gtk.mo" \
    "%{buildroot}%{_datadir}/locale/${locale}/LC_MESSAGES/vega-gtk.mo"
done

%files
%{_bindir}/vega-gtk
%{_bindir}/vega-update-notifier
%{_bindir}/lyra-vega-gtk
%{_datadir}/applications/vega.desktop
%{_sysconfdir}/xdg/autostart/vega-update-notifier.desktop
%{_datadir}/icons/hicolor/scalable/apps/vega.svg
%dir %{_datadir}/gnome-shell
%dir %{_datadir}/gnome-shell/extensions
%{_datadir}/gnome-shell/extensions/updates-indicator@lyraos.org/
%lang(en) %{_datadir}/locale/en_US/LC_MESSAGES/vega-gtk*.mo
%lang(pt_BR) %{_datadir}/locale/pt_BR/LC_MESSAGES/vega-gtk*.mo
%lang(es) %{_datadir}/locale/es_ES/LC_MESSAGES/vega-gtk*.mo

%changelog
