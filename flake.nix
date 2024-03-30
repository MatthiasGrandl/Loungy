{
  inputs = {
    nixpkgs.url = "nixpkgs";
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

        libraries = with pkgs; [
          glib
          openssl_3
          vulkan-headers
          vulkan-loader
          wayland
          wayland-protocols
        ];

        packages = with pkgs; [
          fontconfig
          glib
          libxkbcommon
          openssl_3
          pkg-config
          vulkan-tools
          wayland-scanner
          xorg.libxcb
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
      });
}
