use std::{
    env,
    fs::File,
    io,
    io::BufWriter,
    path::{Path, PathBuf},
};

use flate2::{write::GzEncoder, Compression};
use time::OffsetDateTime;
use xshell::{cmd, Shell};
use zip::{
    write::{FileOptions, SimpleFileOptions},
    DateTime, ZipWriter,
};

use crate::project_root;

const VERSION_STABLE: &str = "0.2";
const VERSION_DEV: &str = "0.3"; // keep this one in sync with `package.json`

pub fn run_dist(sh: &Shell, client_patch_version: Option<String>) -> anyhow::Result<()> {
    let stable = env::var("GITHUB_EVENT_NAME").unwrap_or_default().as_str() == "workflow_dispatch";

    let project_root = project_root();
    let target = Target::get(&project_root);
    let dist = project_root.join("dist");
    sh.remove_path(&dist)?;
    sh.create_dir(&dist)?;

    if let Some(patch_version) = client_patch_version {
        let version = if stable {
            format!("{VERSION_STABLE}.{patch_version}")
        } else {
            format!("{VERSION_DEV}.{patch_version}")
        };

        dist_server(sh, &target)?;
        dist_client(sh, &version, &target)?;
    } else {
        dist_server(sh, &target)?;
    }

    Ok(())
}

fn dist_server(sh: &Shell, target: &Target) -> anyhow::Result<()> {
    let _e = sh.push_env("CARGO_PROFILE_RELEASE_LTO", "thin");

    if target.name.contains("-linux-") {
        unsafe {
            env::set_var("CC", "clang");
        }
    }

    let target_name = &target.name;
    let target_dir = project_root()
        .join("target")
        .join(target_name)
        .join("release");
    cmd!(
        sh,
        "cargo build --release --bin fluent-bit-language-server --target {target_name}"
    )
    .run()?;

    let dst = Path::new("dist").join(&target.artifact_name);
    gzip(&target.server_path, &dst.with_extension("gz"))?;
    if target_name.contains("-windows-") {
        zip(
            &target.server_path,
            target.symbols_path.as_ref(),
            &dst.with_extension("zip"),
        )?;
    }

    Ok(())
}

fn dist_client(sh: &Shell, version: &str, target: &Target) -> anyhow::Result<()> {
    let bundle_path = Path::new("clients").join("vscode").join("server");
    sh.create_dir(&bundle_path)?;
    sh.copy_file(&target.server_path, &bundle_path)?;
    if let Some(symbols_path) = &target.symbols_path {
        sh.copy_file(symbols_path, &bundle_path)?;
    }
    let _d = sh.push_dir("./clients/vscode");

    // TODO
    let mut patch = Patch::new(sh, "./package.json")?;
    patch.replace(
        &format!(r#""version": "{VERSION_DEV}.0-dev""#),
        &format!(r#""version": "{version}""#),
    );
    patch.commit(sh)?;

    Ok(())
}

fn gzip(src_path: &Path, dest_path: &Path) -> anyhow::Result<()> {
    let mut encoder = GzEncoder::new(File::create(dest_path)?, Compression::best());
    let mut input = io::BufReader::new(File::open(src_path)?);
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

fn zip(src_path: &Path, symbols_path: Option<&PathBuf>, dest_path: &Path) -> anyhow::Result<()> {
    let file = File::create(dest_path)?;
    let mut writer = ZipWriter::new(BufWriter::new(file));
    writer.start_file(
        src_path.file_name().unwrap().to_str().unwrap(),
        SimpleFileOptions::default()
            .last_modified_time(
                DateTime::try_from(OffsetDateTime::from(
                    std::fs::metadata(src_path)?.modified()?,
                ))
                .unwrap(),
            )
            .unix_permissions(0o755)
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9)),
    )?;
    let mut input = io::BufReader::new(File::open(src_path)?);
    io::copy(&mut input, &mut writer)?;
    if let Some(symbols_path) = symbols_path {
        writer.start_file(
            symbols_path.file_name().unwrap().to_str().unwrap(),
            SimpleFileOptions::default()
                .last_modified_time(
                    DateTime::try_from(OffsetDateTime::from(
                        std::fs::metadata(src_path)?.modified()?,
                    ))
                    .unwrap(),
                )
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(9)),
        )?;
        let mut input = io::BufReader::new(File::open(symbols_path)?);
        io::copy(&mut input, &mut writer)?;
    }
    writer.finish()?;
    Ok(())
}

struct Target {
    name: String,
    server_path: PathBuf,
    symbols_path: Option<PathBuf>,
    artifact_name: String,
}

impl Target {
    fn get(project_root: &Path) -> Self {
        let name = match env::var("FLB_LS_TARGET") {
            Ok(target) => target,
            _ => {
                if cfg!(target_os = "linux") {
                    "x86_64-unknown-linux-gnu".to_owned()
                } else if cfg!(target_os = "windows") {
                    "x86_64-pc-windows-msvc".to_owned()
                } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
                    "x86_64-apple-darwin".to_owned()
                } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                    "aarch64-apple-darwin".to_owned()
                } else {
                    panic!("Unsupported OS, maybe try setting FLB_LS_TARGET")
                }
            }
        };
        let out_path = project_root.join("target").join(&name).join("release");
        let (exe_suffix, symbols_path) = if name.contains("-windows-") {
            (
                ".exe".into(),
                Some(out_path.join("fluent_bit_language_server.pdb")),
            )
        } else {
            (String::new(), None)
        };
        let server_path = out_path.join(format!("fluent-bit-language-server{exe_suffix}"));
        let artifact_name = format!("fluent-bit-language-server-{name}{exe_suffix}");
        Self {
            name,
            server_path,
            symbols_path,
            artifact_name,
        }
    }
}

struct Patch {
    path: PathBuf,
    original_contents: String,
    contents: String,
}

impl Patch {
    fn new(sh: &Shell, path: impl Into<PathBuf>) -> anyhow::Result<Patch> {
        let path = path.into();
        let contents = sh.read_file(&path)?;
        Ok(Patch {
            path,
            original_contents: contents.clone(),
            contents,
        })
    }

    fn replace(&mut self, from: &str, to: &str) -> &mut Patch {
        assert!(self.contents.contains(from));
        self.contents = self.contents.replace(from, to);
        self
    }

    fn commit(&self, sh: &Shell) -> anyhow::Result<()> {
        sh.write_file(&self.path, &self.contents)?;
        Ok(())
    }
}

impl Drop for Patch {
    fn drop(&mut self) {
        // FIXME: find a way to bring this back
        let _ = &self.original_contents;
        // write_file(&self.path, &self.original_contents).unwrap();
    }
}
