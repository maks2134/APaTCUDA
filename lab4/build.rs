use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/kernels.cu");
    println!("cargo:rerun-if-env-changed=LAB4_CUDA_ARCH");

    let arch = env::var("LAB4_CUDA_ARCH").unwrap_or_else(|_| "sm_86".into());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let kernel = manifest.join("src/kernels.cu");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cubin = out_dir.join("percentiles.cubin");

    let nvcc = find_nvcc().unwrap_or_else(|| PathBuf::from("nvcc"));
    let cl_dir = find_cl_dir();

    let mut cmd = Command::new(&nvcc);
    cmd.arg("-cubin")
        .arg(format!("-arch={arch}"))
        .arg("-o")
        .arg(&cubin)
        .arg(&kernel);
    if let Some(dir) = cl_dir.as_ref() {
        cmd.arg("-ccbin").arg(dir);
    }

    let status = cmd.status().unwrap_or_else(|e| {
        panic!(
            "failed to run nvcc ({nvcc:?}): {e}\n\
             Install CUDA Toolkit and MSVC Build Tools; see SETUP.md"
        );
    });
    if !status.success() {
        panic!("nvcc failed while compiling {} to cubin", kernel.display());
    }
    if !cubin.exists() {
        panic!("nvcc did not produce {}", cubin.display());
    }

    println!("cargo:rustc-env=LAB4_CUBIN={}", cubin.display());
}

fn find_nvcc() -> Option<PathBuf> {
    if let Ok(p) = env::var("CUDA_PATH") {
        let cand = Path::new(&p).join("bin/nvcc.exe");
        if cand.exists() {
            return Some(cand);
        }
    }
    None
}

fn find_cl_dir() -> Option<PathBuf> {
    let vswhere =
        Path::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if vswhere.exists() {
        let out = Command::new(vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-find",
                r"**\Hostx64\x64\cl.exe",
            ])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = text.lines().next() {
            let cl = PathBuf::from(line.trim());
            if cl.exists() {
                return cl.parent().map(|p| p.to_path_buf());
            }
        }
    }

    let roots = [
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        r"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC",
    ];
    for root in roots {
        let root = Path::new(root);
        if !root.exists() {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(root) {
            let mut versions: Vec<_> = rd.filter_map(|e| e.ok()).collect();
            versions.sort_by_key(|e| e.file_name());
            if let Some(v) = versions.last() {
                let dir = v.path().join(r"bin\Hostx64\x64");
                if dir.join("cl.exe").exists() {
                    return Some(dir);
                }
            }
        }
    }
    None
}
