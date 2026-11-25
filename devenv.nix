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
    # WebKit's WebProcess aborts with "Could not create default EGL display:
    # EGL_BAD_PARAMETER" unless glvnd can find an EGL vendor ICD. Point it at
    # the dev shell's own Mesa, which matches this shell's glibc. The host's
    # /run/opengl-driver Mesa is built against the system glibc and fails to
    # load inside the shell.
    export __EGL_VENDOR_LIBRARY_FILENAMES="${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json"
  '';
}
