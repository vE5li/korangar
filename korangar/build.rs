use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use sevenz_rust2::encoder_options::{EncoderOptions, LzmaOptions};
use sevenz_rust2::{ArchiveEntry, ArchiveWriter, EncoderConfiguration, EncoderMethod};

fn check_slangc_availability() {
    let result = Command::new("slangc").arg("-version").output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                println!("cargo:warning=slangc is installed but failed to report version");
            }
        }
        Err(_) => {
            eprintln!("Error: slangc is not available in PATH.");
            eprintln!("slangc is required to compile shaders. You can install it by:");
            eprintln!("  1. Or downloading slang directly from https://github.com/shader-slang/slang/releases");
            eprintln!("  2. Installing the Vulkan SDK from https://vulkan.lunarg.com/");
            eprintln!("After installation, ensure slangc is in your PATH.");
            eprintln!("At least version v2025.18.2 is needed to compile shaders.");
            std::process::exit(1);
        }
    }
}

fn discover_module_files(modules_dir: &Path) -> Vec<PathBuf> {
    let mut module_files = Vec::new();

    if let Ok(entries) = fs::read_dir(modules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "slang") {
                module_files.push(path);
            }
        }
    }

    module_files
}

fn discover_pass_files(passes_dir: &Path) -> Vec<PathBuf> {
    let mut pass_files = Vec::new();

    if let Ok(entries) = fs::read_dir(passes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Ok(pass_entries) = fs::read_dir(&path)
            {
                for pass_entry in pass_entries.flatten() {
                    let pass_file = pass_entry.path();
                    if pass_file.is_file() && pass_file.extension().is_some_and(|ext| ext == "slang") {
                        pass_files.push(pass_file);
                    }
                }
            }
        }
    }

    pass_files
}

fn compile_module(module_file: &Path, output_dir: &Path) -> bool {
    let base_name = module_file.file_stem().unwrap().to_str().unwrap();
    let output_file = output_dir.join(format!("{base_name}.slang-module"));

    let mut cmd = Command::new("slangc");
    cmd.arg("-o").arg(&output_file).arg(module_file);

    let output = cmd.output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                println!("cargo:warning={}", String::from_utf8_lossy(&result.stderr));
                false
            } else {
                true
            }
        }
        Err(error) => {
            println!("cargo:warning={error}");
            false
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_shader(shader_file: &Path, output_dir: &Path, modules_dir: &Path) -> bool {
    let base_name = shader_file.file_stem().unwrap().to_str().unwrap();
    let output_file = output_dir.join(format!("{base_name}.spv"));

    let mut cmd = Command::new("slangc");

    cmd.arg("-target")
        .arg("spirv")
        .arg("-I")
        .arg(modules_dir)
        // Uses column major layout for matrices.
        .arg("-matrix-layout-column-major")
        // Use std430 layout instead of D3D buffer layout for raw buffer load/stores.
        .arg("-fvk-use-gl-layout")
        // This fixes this issue: https://github.com/shader-slang/slang/issues/8549
        // This forces us to define the texture format for a storage texture.
        .arg("-default-image-format-unknown");

    #[cfg(target_os = "macos")]
    {
        // -02 and higher is producing shaders that WGPU can't compile for Metal.
        cmd.arg("-O1");
    }

    #[cfg(not(target_os = "macos"))]
    {
        cmd.arg("-O3");
    }

    cmd.arg("-o").arg(&output_file).arg(shader_file);

    let output = cmd.output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                println!("cargo:warning={}", String::from_utf8_lossy(&result.stderr));
                false
            } else {
                true
            }
        }
        Err(error) => {
            println!("cargo:warning={error}");
            false
        }
    }
}

fn create_shader_archive(output_dir: &Path, passes_output_dir: &Path) {
    let archive_path = output_dir.join("shaders.7z");

    let file = File::create(&archive_path).expect("failed to create archive file");
    let mut writer = ArchiveWriter::new(BufWriter::new(file)).expect("failed to create archive writer");

    writer.set_content_methods(vec![
        EncoderConfiguration::new(EncoderMethod::LZMA2).with_options(EncoderOptions::Lzma(LzmaOptions::from_level(5))),
    ]);

    let entries = fs::read_dir(passes_output_dir).expect("failed to read passes output directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            add_directory_to_archive(&mut writer, &path, passes_output_dir);
        }
    }

    writer.finish().expect("failed to finish archive");
}

fn add_directory_to_archive(writer: &mut ArchiveWriter<BufWriter<File>>, dir_path: &Path, base_path: &Path) {
    let entries = fs::read_dir(dir_path).expect("failed to read directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            add_directory_to_archive(writer, &path, base_path);
        } else if path.extension().is_some_and(|ext| ext == "spv") {
            add_file_to_archive(writer, &path, base_path);
        }
    }
}

fn add_file_to_archive(writer: &mut ArchiveWriter<BufWriter<File>>, file_path: &Path, base_path: &Path) {
    let relative_path = file_path.strip_prefix(base_path).expect("failed to get relative path");
    let archive_path = relative_path.to_string_lossy().replace('\\', "/");

    let file_data = fs::read(file_path).expect("failed to read file");
    let entry = ArchiveEntry::new_file(&archive_path);

    writer
        .push_archive_entry(entry, Some(file_data.as_slice()))
        .expect("failed to add file to archive");
}

fn main() {
    check_slangc_availability();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(manifest_dir);
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    let shader_dir = manifest_path.join("shaders");
    let modules_dir = shader_dir.join("modules");
    let passes_dir = shader_dir.join("passes");
    let output_dir = PathBuf::from(out_dir).join("shaders_compiled");
    let modules_output_dir = output_dir.join("modules");
    let passes_output_dir = output_dir.join("passes");

    println!("cargo:rerun-if-changed={}", shader_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).expect("failed to remove output directory");
    }
    fs::create_dir_all(&output_dir).expect("failed to create output directory");
    fs::create_dir_all(&modules_output_dir).expect("failed to create modules output directory");
    fs::create_dir_all(&passes_output_dir).expect("failed to create passes output directory");

    let mut had_error = false;

    // Phase 1: Precompile modules.
    let module_files = discover_module_files(&modules_dir);
    for module_file in &module_files {
        let success = compile_module(module_file, &modules_output_dir);
        if !success {
            had_error = true;
        }
    }

    // Phase 2: Compile passes.
    let pass_files = discover_pass_files(&passes_dir);
    for pass_file in &pass_files {
        let pass_subdirectory = pass_file.parent().unwrap().strip_prefix(&passes_dir).unwrap_or(Path::new(""));
        let subdirectory = passes_output_dir.join(pass_subdirectory);

        fs::create_dir_all(&subdirectory).expect("failed to create pass output subdirectory");

        let success = compile_shader(pass_file, &subdirectory, &modules_dir);

        if !success {
            had_error = true;
        }
    }

    if had_error {
        std::process::exit(1);
    }

    create_shader_archive(&output_dir, &passes_output_dir);
}
