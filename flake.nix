{
  description = "Nix flake for Malus";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
      in
      {
        packages.malus = pkgs.rustPlatform.buildRustPackage {
          pname = "malus";
          version = cargoToml.package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            glib
            wrapGAppsHook4
            lld
            clang
          ];
          buildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            libadwaita
          ];

          postInstall = ''
            install -Dm644 data/io.github.dissering.Tiltframe.desktop \
              "$out/share/applications/io.github.dissering.Tiltframe.desktop"
            install -Dm644 data/io.github.dissering.Tiltframe.xml \
              "$out/share/mime/packages/io.github.dissering.Tiltframe.xml"
            mkdir -p "$out/share/icons"
            cp -r data/icons/hicolor "$out/share/icons/"
          '';
        };
        packages.default = self.packages.${system}.malus;
      }
    );
}
