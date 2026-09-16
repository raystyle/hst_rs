//! 通用归档与文件工具：sha256 取证、zip / tar.gz 解压、递归复制、宿主
//! os/arch 探测。原在 rmux.rs 的与 rmux 无关部分，D15 收窄后独立成模块
//! 供 install.rs / update.rs 复用。

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// host_os_arch：归档工具的公开入口（行为细则与 marker 见 R002）。
pub fn host_os_arch() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}

/// sha256_file：归档工具的公开入口（行为细则与 marker 见 R002）。
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// extract_zip：归档工具的公开入口（行为细则与 marker 见 R002）。
pub fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("zip {}: {e}", archive.display()))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("zip entry: {e}"))?;
        let name = entry.name().replace('\\', "/");
        if name.contains("..") {
            continue;
        }
        let out = dest.join(name.trim_start_matches('/'));
        if entry.is_dir() || name.ends_with('/') {
            fs::create_dir_all(&out).ok();
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut outfile =
            File::create(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
        io::copy(&mut entry, &mut outfile)
            .map_err(|e| format!("extract {}: {e}", out.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                fs::set_permissions(&out, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }
    Ok(())
}

/// extract_tar_gz：归档工具的公开入口（行为细则与 marker 见 R002）。
pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| format!("tar {}: {e}", dest.display()))
}

/// copy_dir：归档工具的公开入口（行为细则与 marker 见 R002）。
pub fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for ent in fs::read_dir(from)? {
        let ent = ent?;
        let src = ent.path();
        let dst = to.join(ent.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&src) {
                    fs::set_permissions(&dst, meta.permissions())?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_file_matches_known_digest() {
        // Oracle: RFC 4634 / FIPS 180-4 已知答案，"abc" 的 SHA-256。
        let dir = std::env::temp_dir().join(format!("oma-archive-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("abc.txt");
        fs::write(&f, b"abc").unwrap();
        let got = sha256_file(&f).expect("sha256");
        assert_eq!(
            got,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_dir_recurses() {
        let base = std::env::temp_dir().join(format!("oma-copydir-{}", std::process::id()));
        let from = base.join("from");
        let to = base.join("to");
        fs::create_dir_all(from.join("sub")).unwrap();
        fs::write(from.join("sub").join("a.txt"), b"a").unwrap();
        copy_dir(&from, &to).expect("copy");
        assert_eq!(
            fs::read(to.join("sub").join("a.txt")).unwrap(),
            b"a".to_vec()
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn host_os_arch_is_nonempty() {
        let (os, arch) = host_os_arch();
        assert!(!os.is_empty() && !arch.is_empty());
    }
}
