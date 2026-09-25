{ pkgs, ... }:

let
  libraries = with pkgs; [
    webkitgtk_4_1
    gtk3
    cairo
    gdk-pixbuf
    glib
    dbus
    openssl
    librsvg
    sqlite
  ];
in
{
  # Language toolchains
  languages.rust.enable = true;
  languages.javascript = {
    enable = true;
    yarn.enable = true;
  };

  # Tauri v2 build and runtime dependencies
  packages = with pkgs; [
    pkg-config
    gobject-introspection
    cargo-tauri
    makeWrapper
  ] ++ libraries;

  scripts = {
    build-frontend.exec = "yarn build";
    dev-tauri.exec = "cargo tauri dev";
    build-tauri.exec = "cargo tauri build";
  };

  enterShell = ''
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH"
    export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS"
  '';
}
