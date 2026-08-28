{
  description = "Linux development environment for Chikachika";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
    in {
      devShells = forEachSystem (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          runtimeLibraries = with pkgs; [
            gtk3
            libglvnd
            libxkbcommon
            openssl
            wayland
            libx11
            libxcb
            libxcb-util
            libxcb-cursor
            libxcb-image
            libxcb-keysyms
            libxcb-render-util
            libxcb-wm
          ];
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              nodejs_22
              pkg-config
              python3
              rust-analyzer
              rustc
              rustfmt
            ];

            buildInputs = runtimeLibraries;

            shellHook = ''
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibraries}:''${LD_LIBRARY_PATH:-}"
            '';
          };
        });
    };
}
