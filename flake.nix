{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, flake-utils, rust-overlay, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = (import nixpkgs) { inherit system overlays; };
      in
      {
        # Provide a shell with all dependencies needed to build `koragnar::*`
        devShell = pkgs.mkShell
          {
            nativeBuildInputs = with pkgs; [
              (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
                extensions = [ "rust-src" "rust-analyzer" "miri" ];
              }))
              pkg-config
            ];
            buildInputs = with pkgs;
              [
                nixpkgs-fmt
                shaderc
                vulkan-headers
                vulkan-loader
              ] ++ lib.optional stdenv.isDarwin [
                darwin.apple_sdk.frameworks.AppKit
                darwin.apple_sdk.frameworks.CoreGraphics
                darwin.moltenvk
              ] ++ lib.optional stdenv.isLinux [
                libxkbcommon
                vulkan-validation-layers
              ];
            # For any tools that need to see the rust toolchain src
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            shellHook =
              ''
                alias nixmft=nixpkgs-fmt
              ''
              # For darwin, explicitly set our LD_LIBRARY_PATH, otherwise `cargo run` will not find paths in e.g. `/nix/store`
              + (pkgs.lib.strings.optionalString pkgs.stdenv.isDarwin ''
                export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:${
                  pkgs.lib.strings.makeLibraryPath
                  [
                    pkgs.darwin.apple_sdk.frameworks.AppKit
                    pkgs.darwin.apple_sdk.frameworks.CoreGraphics
                    pkgs.darwin.moltenvk
                  ]}
              '');
          };
        # If we want to `nix build` and provide a derivation, we can use
        # `naersk`, e.g.
        #   defaultPackage = naersk'.buildPackage {
        #      src = ./.;
        #   };
        #   app.default = { ... }
      }
    );
}
