# .tools：项目自定义脚本工具

> 角色：项目管理与操作过程中**按需自定义、临时编写**的 PowerShell、Python、Rust 工具及代码的归档目录，进 git。用完即归档进此目录，不散落仓库根或会话里。

## 使用规则

1. **归档时机**：会话中为完成某操作临时写出的脚本，若具备复用价值（第二次会用到的），当轮收尾前移入 `.tools\` 并加 PEP 723 头（Python）或用法注释（ps1）；纯一次性的留在对话里不进仓。
2. **Python 统一 uv 载体**：脚本头部带 `# /// script` 内联元数据，运行用 `uv run --script .tools\xxx.py`（对齐 ohmypwsh 模式；不建 venv、不装依赖进环境）。py 选库：关键词搜索只有网页、已知名字走 PyPI JSON API 加 pypistats；ps 模块装本机统一经 ohmypwsh `psmodule.ps1`、不自行 Install-Module（psgallery 查名）。
3. **命名**：小写连字符加用途动词或名词（`md-ref-scan.py`、`md-replace.py`）；ps1 同风格；Rust 专用工具（若有）先 `cargo new` 独立子目录再入 `.tools\`。
4. **工具自述**：每个脚本 docstring 写清用法、参数、退出码；改动同步本 README 清单。
5. **门禁联动**：`md-ref-scan.py` 在文档结构大改（改名、编号、移目录）后必跑；退出码非 0 即有断链，先修后提交。

## 工具清单

| 工具 | 用途 | 用法 |
| --- | --- | --- |
| `md-ref-scan.py` | 全仓 markdown 仓内路径引用断链扫描（结构大改后的回归门禁） | `uv run --script .tools/md-ref-scan.py [--root docs] [--allow 豁免.txt]`；退出码 0/1 |
| `md-heading-scan.py` | 标题括号规范扫描（标题干净机检项；代码围栏内的注释不计） | `uv run --script .tools/md-heading-scan.py [--root docs]`；退出码 0/1 |
| `mdcharlint.py` | 四类禁用字符检查（掩豁免区后扫，中文标点白名单放行） | `uv run --script .tools/mdcharlint.py 文件.md ...`；退出码 0/1 |
| `md-replace.py` | 中文与反斜杠路径安全的字面批量替换（规避 sed 转义坑，见 M023） | `uv run --script .tools/md-replace.py --glob 'docs/**/*.md' --map 映射.txt [--dry]` |

## 历史注记

- 两个首发工具由 2026-08-31 文档整编与全量 REVIEW 中验证过的内联脚本正式化（断链扫描器当轮扫出 21 处断链并修复）。
- D15（2026-09-08）去编排后删除两件编排时代工具：`review-round.py`（agent 轮换接力 review，消费面 `oma send/task` 已移除）与 `share-view-probe.py`（rmux web-share 探针）。历史见 git。
- `cross-test.sh`：全平台实弹测试矩阵（REQ-007），**5端4机**基建原语（wsl 加 lan-win 加 lan-mac 加 lan-ubuntu 加 lan-linux；wsl 与 lan-win 同宿主机）——本地构建 linux/mac/windows-gnu 三产物，wsl 就地跑、scp 到 lan-ubuntu 与 lan-linux 与 lan-mac 冒烟，lan-win 走 127.0.0.1 回环 powershell 冒烟。用法 `bash .tools/cross-test.sh`。
| `release.ps1` | 本地发布链（ADR-0007：版本闸加预检加测试与 md 与 aidoc 闸加三端构建打包加解包冒烟加 gh 直发 --latest；`-DryRun` 到冒烟为止；`-SkipWinSmoke` 显式豁免 win 冒烟） | `pwsh -NoProfile -File .tools/release.ps1 [-MacHost lan-mac] [-DryRun] [-SkipWinSmoke]` |
