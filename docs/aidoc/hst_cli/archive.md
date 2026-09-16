# hst-cli::archive

通用归档工具：sha256 校验、zip/tar.gz 解包、目录复制、host os/arch。
通用归档与文件工具：sha256 取证、zip / tar.gz 解压、递归复制、宿主
os/arch 探测。原在 rmux.rs 的与 rmux 无关部分，D15 收窄后独立成模块
供 install.rs / update.rs 复用。

## Functions

- `copy_dir` — copy_dir：归档工具的公开入口（行为细则与 marker 见 R002）。
- `extract_tar_gz` — extract_tar_gz：归档工具的公开入口（行为细则与 marker 见 R002）。
- `extract_zip` — extract_zip：归档工具的公开入口（行为细则与 marker 见 R002）。
- `host_os_arch` — host_os_arch：归档工具的公开入口（行为细则与 marker 见 R002）。
- `sha256_file` — sha256_file：归档工具的公开入口（行为细则与 marker 见 R002）。

