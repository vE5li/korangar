# GPU Shaders

We use the Slang shader language to better re-use code. We compile them into SPIR-V files and load them later at
runtime. These shaders are compiled with `slangc` by the `build.rs` script:

- `modules`: Reusable code that is compiled by `slangc` into their own compiled modules. A module need to have a file
  in the root of modules, but can be split into sub-folder / sub-files using `__include` inside the main module file.
- `passes`: The actual shader files that will be compiled to SPIR-V file and loaded by WGPU at runtime. Each of our
  passes has its own sub-folder, with all shaders of the pipelines in their own file.

## WGSL files

We still have some WGSL files left, until some issues are fixed upstream:

- Problem sampling depth texture arrays and cubes (Don't know yet if Slang or WGPU problem. Needs more debugging).
- Shaders tha sample integer based textures: https://github.com/shader-slang/slang/issues/8549

All WGSL shaders will eventually be ported to Slang once the outstanding issues are fixed.
