{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    flake-utils,
    rust-overlay,
    nixpkgs,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = (import nixpkgs) {inherit system overlays;};
      # Can be removed once slangc 2025.18.2 is officially packaged for nix.
      shader-slang-git = pkgs.shader-slang.overrideAttrs (oldAttrs: rec {
        version = "2025.19.1";

        src = pkgs.fetchFromGitHub {
          owner = "shader-slang";
          repo = "slang";
          tag = "v${version}";
          hash = "sha256-mbtyvPM3dtIZRU9dWMCZ/XCf2mDAPuJMhagMLgFsdWI=";
          fetchSubmodules = true;
        };

        # Patches are no longer required.
        patches = [];

        # Build using the included miniz and lz4 dependencies.
        cmakeFlags =
          map (
            flag:
              if pkgs.lib.hasPrefix "-DSLANG_USE_SYSTEM_MINIZ=" flag
              then "-DSLANG_USE_SYSTEM_MINIZ=OFF"
              else if pkgs.lib.hasPrefix "-DSLANG_USE_SYSTEM_LZ4=" flag
              then "-DSLANG_USE_SYSTEM_LZ4=OFF"
              else flag
          )
          oldAttrs.cmakeFlags
          ++ [
            "-DSLANG_ENABLE_TESTS=OFF"
            "-DSLANG_ENABLE_EXAMPLES=OFF"
            "-DSLANG_ENABLE_GFX=OFF"
          ];
      });
    in {
      formatter = pkgs.alejandra;

      # Provide a shell with all dependencies needed to build `koragnar::*`
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          pkg-config
        ];
        buildInputs = with pkgs;
          [libpcap nasm nixpkgs-fmt openssl shaderc shader-slang-git vulkan-headers vulkan-loader]
          ++ lib.optional stdenv.isDarwin [
            apple-sdk
            moltenvk
          ]
          ++ lib.optional stdenv.isLinux [
            alsa-lib.dev
            libxkbcommon
            vulkan-validation-layers
          ];

        # For any tools that need to see the rust toolchain src
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        shellHook =
          ""
          # For darwin, explicitly set our LD_LIBRARY_PATH, otherwise `cargo run` will not find paths in e.g. `/nix/store`
          + (pkgs.lib.strings.optionalString pkgs.stdenv.isDarwin ''
            export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:${
              pkgs.lib.strings.makeLibraryPath [
                pkgs.apple-sdk
                pkgs.darwin.moltenvk
              ]
            };
          '')
          + (pkgs.lib.strings.optionalString pkgs.stdenv.isLinux ''
            export VULKAN_SDK="$VULKAN_SDK:${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d:${pkgs.vulkan-headers}";
            export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${
              pkgs.lib.strings.makeLibraryPath [
                pkgs.wayland
                pkgs.vulkan-loader
                pkgs.libxkbcommon
              ]
            };
          '');
      };
      # If we want to `nix build` and provide a derivation, we can use
      # `naersk`, e.g.
      #   defaultPackage = naersk'.buildPackage {
      #      src = ./.;
      #   };
      #   app.default = { ... }
    });
}
