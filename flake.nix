{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        libraries = with pkgs; (pkgs.lib.strings.optionalString stdenv.isLinux [
          glib
          openssl_3
          vulkan-headers
          vulkan-loader
          wayland
          wayland-protocols
        ]) ++ [];

        packages = with pkgs; (pkgs.lib.strings.optionalString stdenv.isLinux [
          fontconfig
          glib
          libxkbcommon
          openssl_3
          pkg-config
          vulkan-tools
          wayland-scanner
          xorg.libxcb
        ]) ++ [
          (pkgs.rust-bin.stable.latest.default.override
            { extensions = [ "rust-src" ]; })
        ];
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = packages;
          shellHook =
            ''
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            '';
        };
        packages = rec {
          loungy = pkgs.rustPlatform.buildRustPackage {
            name = "loungy";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
          };
          default = loungy;
        };
        apps = rec {
          loungy = flake-utils.lib.mkApp { drv = self.packages.${system}.loungy; };
          default = loungy;
        };
      }
    );
}
