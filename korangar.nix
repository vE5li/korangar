{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = [
    pkgs.shaderc
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-validation-layers
    pkgs.libxkbcommon
  ];

  LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.vulkan-loader}/lib:${pkgs.libxkbcommon}/lib";
  VULKAN_SDK = "$VULKAN_SDK:${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
}
