use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use glob::glob;
use rayon::prelude::*;
use std::env;
use std::fs::{read_to_string, write};
use std::path::PathBuf;
use std::time::SystemTime;

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

impl ShaderData {
    pub fn load(src_path: PathBuf) -> Result<Self> {
        let extension = src_path
            .extension()
            .context("File has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        let kind = match extension {
            "vert" | "vs" => shaderc::ShaderKind::Vertex,
            "frag" | "fs" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", src_path.display()),
        };

        let src = read_to_string(src_path.clone())?;
        let spv_path = src_path.with_extension(format!("{}.spv", extension));

        Ok(Self {
            src,
            src_path,
            spv_path,
            kind,
        })
    }

    fn src_modification_time(&self) -> std::io::Result<SystemTime> {
        let metadata = std::fs::metadata(&self.src_path)?;
        metadata.modified()
    }

    fn spv_modification_time(&self) -> std::io::Result<SystemTime> {
        let metadata = std::fs::metadata(&self.spv_path)?;
        metadata.modified()
    }

    pub fn spv_out_of_date(&self) -> bool {
        match (self.src_modification_time(), self.spv_modification_time()) {
            (Ok(src_mod_time), Ok(spv_mod_time)) => {
                println!("src:{:?} src_mod_time: {:?} spv:{:?} spv_mod_time:{:?} cmp:{:?}",
                self.src_path, src_mod_time, self.spv_path, spv_mod_time, (src_mod_time > spv_mod_time)
            );
                src_mod_time >  spv_mod_time
            }
            _ => true
        }
    }
}

fn main() -> Result<()> {
    // This tells cargo to rerun this script if something in /src/ changes.
    println!("cargo:rerun-if-changed=src/*");

    // Collect all shaders recursively within /src/
    let mut shader_paths = Vec::new();

    shader_paths.extend(glob("./src/**/*.vert")?);
    shader_paths.extend(glob("./src/**/*.vs")?);
    shader_paths.extend(glob("./src/**/*.frag")?);
    shader_paths.extend(glob("./src/**/*.fs")?);
    shader_paths.extend(glob("./src/**/*.comp")?);

    let shaders = shader_paths
        .into_par_iter()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Vec<Result<_>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>();

    let mut compiler = shaderc::Compiler::new().context("Unable to create shader compiler")?;

    // This can't be parallelized. The [shaderc::Compiler] is not thread safe.
    for shader in shaders? {
        if shader.spv_out_of_date() {
            println!("Compiling out-of-date SPV for: {:?}", &shader.src_path);
            let compiled = compiler.compile_into_spirv(
                &shader.src,
                shader.kind,
                &shader.src_path.to_str().unwrap(),
                "main",
                None,
            )?;
            write(shader.spv_path, compiled.as_binary_u8())?;
        }
    }

    // Copy resource files (models, textures, etc)
    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let paths_to_copy = vec!["res/"];
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
