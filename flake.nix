{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    korangar-rathena.url = "github:vE5li/korangar-rathena";
  };

  outputs = {
    self,
    flake-utils,
    rust-overlay,
    nixpkgs,
    korangar-rathena,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay) korangar-rathena.overlays.default];
      pkgs = (import nixpkgs) {inherit system overlays;};

      test-log-file = "/tmp/korangar-packet-test.log";
      run-packet-test = pkgs.writeShellScriptBin "run-packet-test" ''
        ${pkgs.lib.getExe pkgs.rathena-test-20220406} > ${test-log-file} 2>&1 &
        RATHENA_PID=$!

        cargo build --bin packet_test

        while ! grep -q "test rAthena is running" ${test-log-file} 2> /dev/null; do
          echo "Waiting for rAthena..."
          sleep 0.5
        done

        echo "rAthena is ready"
        sleep 5

        cargo run --bin packet_test

        kill $RATHENA_PID
      '';
    in {
      formatter = pkgs.alejandra;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          pkg-config
        ];
        buildInputs = with pkgs;
          [libpcap nasm nixpkgs-fmt openssl shaderc shader-slang vulkan-headers vulkan-loader run-packet-test]
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
    });
}
