{
  description = "A development environment for rcarve-iced (Rust GUI application)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (rust-overlay.overlays.default) ];
      };
      
      # Define the libraries we need at RUNTIME
      runtimeLibs = with pkgs; [
        wayland
        libxkbcommon
        mesa
        vulkan-loader
        xorg.libX11
        xorg.libXcursor
        xorg.libXi
        xorg.libXrandr
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          # Rust Toolchain
          pkgs.rust-bin.stable.latest.default
          # Build tools
          pkgs.pkg-config
          pkgs.gcc
        ] ++ runtimeLibs; # Add runtime libs to build inputs too

        shellHook = ''
          # 1. Setup LD_LIBRARY_PATH so the binary finds libraries at runtime
          export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath runtimeLibs}
          
          # 2. Debugging: Print to ensure variables are correct
          echo "------------------------------------------------"
          echo "👻 Nix Environment Loaded"
          echo "   WAYLAND_DISPLAY: $WAYLAND_DISPLAY"
          echo "   XDG_RUNTIME_DIR: $XDG_RUNTIME_DIR"
          echo "------------------------------------------------"
        '';
      };
    };
}
