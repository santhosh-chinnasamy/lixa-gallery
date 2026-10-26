# Lixa Gallery

Circle the keepers.

Open a folder from a shoot, mark the frames worth keeping, and copy or move them out. A small desktop app for culling photos. Nothing is uploaded.

**Site:** https://santhosh-chinnasamy.github.io/lixa-gallery/

![Lixa Gallery Logo](./src-tauri/icons/128x128.png 'Lixa Gallery Logo')

## Installation

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/santhosh-chinnasamy/lixa-gallery?label=Latest%20Version)](https://github.com/santhosh-chinnasamy/lixa-gallery/releases/latest)

Installers for macOS (Apple silicon and Intel), Windows, and Linux are on the [site](https://santhosh-chinnasamy.github.io/lixa-gallery/#install). The button for your machine downloads that release asset directly.

The site is published with GitHub Pages from [`.github/workflows/pages.yml`](.github/workflows/pages.yml).

### Nix / NixOS

#### Run directly (using Cachix binary cache)

```bash
nix run --accept-flake-config github:santhosh-chinnasamy/lixa-gallery
```

#### Install on NixOS (via Flakes)

Add `lixa-gallery` to your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    lixa-gallery.url = "github:santhosh-chinnasamy/lixa-gallery";
  };

  outputs = { nixpkgs, lixa-gallery, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [
            lixa-gallery.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```

Pre-built binaries are provided via [Cachix](https://app.cachix.org/cache/lixa-gallery).

## Keyboard shortcuts

The full list — gallery, viewer, and slideshow — is on the [site](https://santhosh-chinnasamy.github.io/lixa-gallery/#shortcuts). It is taken from the shortcut handlers in the app.

## Screenshots / Demo

[![Demo Video](https://img.shields.io/badge/Demo%20Video-Youtube-FF0000?style=plastic&logo=youtube&logoColor=FF0000&link=https://youtu.be/F1yKYnQ873I)](https://youtu.be/F1yKYnQ873I)

![Gallery](./site/assets/shots/gallery.jpg 'Lixa Gallery — folder grid')

![Viewer](./site/assets/shots/viewer.jpg 'Lixa Gallery — full-size viewer')

![Export](./site/assets/shots/export.jpg 'Lixa Gallery — export favourites')

## Supports

- [x] Mac
- [x] Linux
- [x] Windows

## Contribution

Please read the [contributing guidelines](CONTRIBUTING.md) to setup your development machine and proceed.

## Attributions

- Logo generated using svg from <a href="https://www.svgrepo.com" target="_blank">SVG Repo</a>

<video src="./assets/demo.mp4" controls="" alt="Lixa Gallery Demo" title="Lixa Gallery Demo" height="400" width="auto" />
