{
  description = "A Nix-flake-based development environment for lixa-gallery (Tauri v2)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        pname = "lixa-gallery";
        version = "0.7.0";
        src = ./.;

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

        nativeBuildInputs = with pkgs; [
          pkg-config
          gobject-introspection
          cargo
          rustc
          nodejs
          yarn
          makeWrapper
        ];

        buildInputs = with pkgs; [
          at-spi2-atk
          atkmm
          cairo
          gdk-pixbuf
          glib
          gtk3
          harfbuzz
          librsvg
          libsoup_3
          pango
          webkitgtk_4_1
          openssl
          sqlite
        ];

        # 1. FIXED-OUTPUT DERIVATION FOR FRONTEND
        # This builds the frontend assets in a pure way.
        frontend-assets = pkgs.stdenv.mkDerivation {
          pname = "${pname}-frontend";
          inherit version src;

          # --- THE HASH PART ---
          # Update this hash whenever package.json or yarn.lock changes.
          outputHash = "sha256-QRCOGsGjvXo0JMtryPGkwnLD6nDOGqk8RdXpekggZ5c="; 
          outputHashAlgo = "sha256";
          outputHashMode = "recursive";
          # ---------------------

          nativeBuildInputs = [ pkgs.nodejs pkgs.yarn pkgs.cacert ];

          buildPhase = ''
            export HOME=$TMPDIR
            export NODE_EXTRA_CA_CERTS=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
            
            yarn install --immutable
            yarn build
          '';

          installPhase = ''
            cp -r build $out
          '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit pname version src nativeBuildInputs buildInputs;

          cargoLock = {
            lockFile = ./src-tauri/Cargo.lock;
          };

          buildAndTestSubdir = "src-tauri";

          postPatch = ''
            ln -s src-tauri/Cargo.lock Cargo.lock
          '';

          preBuild = ''
            # Copy the pre-built frontend assets so Tauri can bundle them
            mkdir -p ../build
            cp -r ${frontend-assets}/* ../build/
          '';

          postInstall = ''
            wrapProgram $out/bin/lixa-gallery \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath libraries}" \
              --prefix XDG_DATA_DIRS : "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}"
          '';

          doCheck = false;

          meta = with pkgs.lib; {
            description = "Select your favorite photos and export";
            homepage = "https://github.com/santhosh-chinnasamy/lixa-gallery";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };
      }
    );
}
