# Lixa Gallery

Select your favorite photos and export

![Lixa Gallery Logo](./src-tauri/icons/128x128.png 'Lixa Gallery Logo')
## Installation

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/santhosh-chinnasamy/lixa-gallery?label=Latest%20Version)](https://github.com/santhosh-chinnasamy/lixa-gallery/releases/latest)

### Standard
Download latest version from [release](https://github.com/santhosh-chinnasamy/lixa-gallery/releases) page for your OS and run the installer.

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

## Keyboard Shortcuts

- `o` - Open / Select folder
- `l` - toggle image as favorite in preview mode or in gallery mode
- `e` - export favorites
- `F11` - toggle fullscreen

## Screenshots / Demo

[![Demo Video](https://img.shields.io/badge/Demo%20Video-Youtube-FF0000?style=plastic&logo=youtube&logoColor=FF0000&link=https://youtu.be/F1yKYnQ873I)](https://youtu.be/F1yKYnQ873I)

![Lixa Gallery Screenshot](./assets/screen-3.png 'Lixa Gallery Screenshot')

## Supports

- [x] Mac
- [x] Linux
- [x] Windows

## Contribution

Please read the [contributing guidelines](CONTRIBUTING.md) to setup your development machine and proceed.

## Attributions

- Logo generated using svg from <a href="https://www.svgrepo.com" target="_blank">SVG Repo</a>


<video src="./assets/demo.mp4" controls="" alt="Lixa Gallery Demo" title="Lixa Gallery Demo" height="400" width="auto" />
