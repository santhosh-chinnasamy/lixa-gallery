{
  description = "Lixa Gallery - Photo management desktop application";

  nixConfig = {
    extra-substituters = [
      "https://lixa-gallery.cachix.org"
    ];
    extra-trusted-public-keys = [
      "lixa-gallery.cachix.org-1:cy41vSEjVo4t1+nLSz4B8/Rd261rKvo0vp8BBwC692w="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = forAllSystems (system: nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = pkgsFor.${system};
          packageJson = builtins.fromJSON (builtins.readFile ./package.json);
          pname = "lixa-gallery";
          version = packageJson.version;
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
            copyDesktopItems
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

          # Fixed-output derivation for building frontend assets
          frontend-assets = pkgs.stdenv.mkDerivation {
            pname = "${pname}-frontend";
            inherit version src;

            # Update this hash whenever package.json or yarn.lock changes.
            outputHash = "sha256-Sh+AdV0HJNc0+F7Sg2PhezJuTydm0KwzRVSALOVdJzY=";
            outputHashAlgo = "sha256";
            outputHashMode = "recursive";

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
          frontend = frontend-assets;

          default = pkgs.rustPlatform.buildRustPackage {
            inherit pname version src nativeBuildInputs buildInputs;

            cargoLock = {
              lockFile = ./src-tauri/Cargo.lock;
            };

            buildAndTestSubdir = "src-tauri";

            buildFeatures = [ "tauri/custom-protocol" ];

            postPatch = ''
              ln -s src-tauri/Cargo.lock Cargo.lock
            '';

            preBuild = ''
              mkdir -p build
              cp -r ${frontend-assets}/* build/
            '';

            desktopItems = [
              (pkgs.makeDesktopItem {
                name = "lixa-gallery";
                exec = "lixa-gallery";
                icon = "lixa-gallery";
                comment = "Select your favorite photos and export";
                desktopName = "Lixa Gallery";
                categories = [ "Graphics" "Photography" "Viewer" ];
              })
            ];

            postInstall = ''
              install -Dm644 src-tauri/icons/128x128.png $out/share/icons/hicolor/128x128/apps/lixa-gallery.png
              install -Dm644 src-tauri/icons/32x32.png $out/share/icons/hicolor/32x32/apps/lixa-gallery.png

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
              mainProgram = "lixa-gallery";
            };
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/lixa-gallery";
        };
      });
    };
}
