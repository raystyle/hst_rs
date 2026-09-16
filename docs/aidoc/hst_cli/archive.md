# hst-cli::archive

的通用归档工具面（细则见 R002 与模块文档）。
通用归档与文件工具：sha256 取证、zip / tar.gz 解压、递归复制、宿主
os/arch 探测。原在 rmux.rs 的与 rmux 无关部分，D15 收窄后独立成模块
供 install.rs / update.rs 复用。

## Functions

- `copy_dir` — # Errors
- `extract_tar_gz` — # Errors
- `extract_zip` — # Errors
- `host_os_arch` — 归档的host_os_arch面（细则见 R002 与模块文档）。
- `sha256_file` — # Errors

