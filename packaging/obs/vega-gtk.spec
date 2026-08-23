# Empacotamento para o openSUSE Build Service (home:rodrigosbrito:vega).
# Cópia de packaging/opensuse/vega.spec adaptada só no Source0/%setup pra
# bater com o tarball que o _service (tar_scm) deste mesmo diretório
# gera — nome com sufixo de versão e diretório interno próprio, ao invés
# do tar "achatado" usado pelo empacotamento local. Resto do spec é
# idêntico ao de packaging/opensuse/.
#
# Version literal (não %%{version}/%%define) — o serviço set_version deste
# diretório faz substituição textual simples na linha "Version:" e não
# entende macro, então precisa achar um valor literal aqui pra reescrever.
Name:           vega-gtk
Version:        0
Release:        1%{?dist}
Summary:        Centro de controle para Linux
License:        GPL-3.0-only
URL:            https://github.com/lyra-os-linux/vega
Source0:        vega-src-%{version}.tar
# vendor.tar.gz gerado pelo _service cargo_vendor (rede exigida, que a VM
# de build do OBS não tem — sem isso, "cargo build" trava tentando baixar
# crates de index.crates.io e falha). Traz .cargo/config.toml + Cargo.lock
# + vendor/ prontos pra extrair na raiz do workspace.
Source1:        vendor.tar.gz

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
%setup -q -n vega-src-%{version}
# .cargo/config.toml + vendor/ vão na raiz do workspace, junto do
# Cargo.toml — é onde o cargo procura por padrão.
# O vendor.tar.gz pode ter sido gerado numa release anterior. Preserve o
# Cargo.lock da tag atual; o tar fornece apenas a configuração offline e os
# crates vendorizados.
tar --anchored --exclude=Cargo.lock -xzf %{SOURCE1}

%build
cd vega-gtk
VEGA_VERSION=%{version} cargo build --release --locked --offline

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
