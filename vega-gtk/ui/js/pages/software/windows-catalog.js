// Apps do Windows (Vega → Software → Apps do Windows), a partir da discussão
// #356. A seção "Base" traz o que é preciso para rodar programas do Windows;
// o botão "Habilitar apps do Windows" instala o que faltar dela. Os apps
// (`origin: "windows"`) são verbos do winetricks ou receitas do Vega
// (instalador oficial com SHA-256 fixado), instalados como o usuário num
// prefixo só do Vega; backend/windows_apps.rs só aceita os ids de VERBS e
// RECIPES. Enquanto a base ou o suporte a 32 bits faltarem, ficam
// desabilitados.
//
// Sem logos: são marcas dos fabricantes. Bottles reaproveita o ícone livre
// já usado em Populares.

import { N_ } from "../../i18n.js";

/** Pacotes da base: todos precisam estar instalados para liberar os apps. */
export const WINDOWS_BASE = [
  { name: "Wine", origin: "official", id: "wine", text: N_("Roda programas do Windows no Linux") },
  { name: "Wine Mono", origin: "official", id: "wine-mono", text: N_("Suporte a programas .NET no Wine") },
  { name: "Wine Gecko", origin: "official", id: "wine-gecko", text: N_("Suporte a páginas web dentro de programas do Windows") },
  { name: "Winetricks", origin: "official", id: "winetricks", text: N_("Instala programas e bibliotecas do Windows no Wine") },
  // O winetricks extrai instaladores compactados (Winamp, AutoHotkey) com o 7z.
  { name: "7-Zip (7z)", origin: "official", id: "7zip", text: N_("Extrai os instaladores compactados para o Winetricks") },
];

export const WINDOWS = [
  {
    id: "base",
    title: N_("Base"),
    apps: [
      ...WINDOWS_BASE,
      { name: "Bottles", origin: "flathub", id: "com.usebottles.bottles", logo: "com.usebottles.bottles", text: N_("Organiza programas e jogos do Windows em garrafas separadas") },
    ],
  },
  {
    id: "tools",
    title: N_("Utilitários"),
    gated: true,
    apps: [
      { name: "7-Zip", origin: "windows", id: "7zip", text: N_("Compactador de arquivos") },
      { name: "WinRAR", origin: "windows", id: "winrar", text: N_("Compactador de arquivos RAR e ZIP") },
      { name: "Notepad++", origin: "windows", id: "npp", text: N_("Editor de texto e código") },
      { name: "WinMerge", origin: "windows", id: "winmerge", text: N_("Compara arquivos e pastas lado a lado") },
      { name: "AutoHotkey", origin: "windows", id: "autohotkey", text: N_("Atalhos e automações de teclado") },
    ],
  },
  {
    id: "reading",
    title: N_("Leitura e PDF"),
    gated: true,
    apps: [
      { name: "SumatraPDF", origin: "windows", id: "sumatrapdf", text: N_("Leitor leve de PDF, e-books e quadrinhos") },
      { name: "Kindle", origin: "windows", id: "kindle", text: N_("Leitor de e-books da Amazon") },
      { name: "Adobe Digital Editions", origin: "windows", id: "adobe_diged4", text: N_("E-books com DRM de bibliotecas e livrarias") },
      { name: "Kobo", origin: "windows", id: "kobo", text: N_("Leitor de e-books da Kobo") },
    ],
  },
  {
    id: "graphics",
    title: N_("Imagem e desenho"),
    gated: true,
    apps: [
      { name: "Paint", origin: "windows", id: "mspaint", text: N_("O Paint clássico do Windows") },
      { name: "SketchUp 8", origin: "windows", id: "sketchup", text: N_("Modelagem 3D (versão gratuita de 2012)") },
    ],
  },
  {
    id: "network",
    title: N_("Redes e TI"),
    gated: true,
    apps: [
      { name: "Winbox", origin: "windows", id: "winbox", text: N_("Administra roteadores MikroTik") },
      { name: "HeidiSQL", origin: "windows", id: "heidisql", text: N_("Cliente de MySQL, MariaDB, PostgreSQL e SQL Server") },
    ],
  },
  {
    id: "other",
    title: N_("Música, jogos e finanças"),
    gated: true,
    apps: [
      { name: "Winamp", origin: "windows", id: "winamp", text: N_("Player de música clássico") },
      { name: "Ubisoft Connect", origin: "windows", id: "ubisoftconnect", text: N_("Loja e biblioteca de jogos da Ubisoft") },
      { name: "MetaTrader 4", origin: "windows", id: "mt4", text: N_("Plataforma de negociação no mercado financeiro") },
    ],
  },
];
