{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    shaderc
    vulkan-headers
    vulkan-loader
    vulkan-validation-layers
    libxkbcommon
  ];

  LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.vulkan-loader}/lib:${pkgs.libxkbcommon}/lib";
  VULKAN_SDK="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  # VULKAN_SDK="${pkgs.vulkan-headers}";
}
