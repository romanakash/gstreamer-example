{
  inputs = {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
      flake-parts.url = "github:hercules-ci/flake-parts";
      systems.url = "github:nix-systems/default";
    };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
        systems = import inputs.systems;
      
        perSystem = { config, self', pkgs, lib, system, ... }:
          let
            cargoToml = builtins.fromToml (builtins.readFile ./cargoToml);
          in 
          {
              # Rust pacakge
              packages.default = pkgs.rustPlatform.buildRustPackage {
                  inherit(cargoToml.package) name version;
                  src = ./.;
                  cargoLock.lockFile = ./Cargo.lock;
                };
              
              # Rust dev env
              devShells.default = pkgs.mkShell {
                  shellHook = ''
                    export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
                  '';
                  nativeBuildInputs = with pkgs; [
                      just
                      rustc 
                      cargo 
                      cargo-watch
                      rust-analyzer


                      # Video/Audio data composition framework tools like "gst-inspect", "gst-launch" ...
                      gst_all_1.gstreamer
                      # Common plugins like "filesrc" to combine within e.g. gst-launch
                      gst_all_1.gst-plugins-base
                      # Specialized plugins separated by quality
                      gst_all_1.gst-plugins-good
                      gst_all_1.gst-plugins-bad
                      gst_all_1.gst-plugins-ugly
                      # Plugins to reuse ffmpeg to play almost every video format
                      gst_all_1.gst-libav
                      # Support the Video Audio (Hardware) Acceleration API
                      gst_all_1.gst-vaapi
                    ];
                };
          };
    };
}
