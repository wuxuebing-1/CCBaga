# Windows 发布产物来源核验

核验时间：2026-10-21 02:01 UTC（美国中部时间 2026-10-20 晚间）。

结论：`v0.2.0-windows.1` 的公开安装包与成功 Windows CI 安装包**不一致**。
本记录不证明安装包有恶意行为，也不替代 Windows 10/11 实机安装验收。未修改 Release 或安装包。

## 固定证据

- [Release](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1)，Release ID `410368282`，安装包 Asset ID `633153665`。
- [成功 Windows 运行](https://github.com/wuxuebing-1/CCBaga/actions/runs/38224529893)，attempt `1`，源提交和标签解析均为 `376bb7f4aeb8f2dc660d32119767aad326c89d40`。
- [Artifact](https://github.com/wuxuebing-1/CCBaga/actions/runs/38224529893/artifacts/11737001917)：`CCBaga-Windows-x64`，ID `11737001917`。
- Release EXE 上传完成 `2026-10-12T08:29:23Z`，CI 开始 `08:35:48Z`，Release 发布 `08:43:58Z`，Artifact 创建 `08:51:18Z`，CI 完成元数据更新 `08:51:26Z`。

| 实际下载并计算的对象 | 字节数 | SHA-256 |
| --- | ---: | --- |
| Release `CCBaga-A_0.2.0_x64-setup.exe` | 139029303 | `c7a4b7df48d28e706f77a36f0fdf3c4d584561fa1aac27504f9c80079e8966c3` |
| Artifact ZIP 内同名 EXE | 139115301 | `d680429d6b56a59929a11fe46820d891ae4c7c5af611e6e478786e13c3bbf619` |
| 原始 Artifact ZIP | 139137420 | `23f0bca22e76d9d7eecd5664db5770988b67e1aa0a99a1e93fd011767ac680c3` |

ZIP 摘要与 GitHub Artifact 元数据及上传日志吻合。ZIP 内 `SHA256SUMS.txt` 与重新计算的 CI EXE 摘要吻合。Release EXE 摘要与 GitHub Asset 元数据吻合。比较脚本返回 `1`，`identical: false`。

## 载荷与构建差异

使用 7-Zip 26.00 只读解开两份 NSIS，均成功，分别产生 111 个文件；109 个文件路径和字节完全相同，包括完整浏览器引擎、双入口 UI、守卫和许可。两个差异文件如下：

| 文件 | Release | CI |
| --- | --- | --- |
| `requirement-french.exe` | 10791424 字节；`0c24c4b6b8908cfea1c7e0cd6f83b1c5c172dce5567917f0f82b0aea43ba913c` | 10792448 字节；`7362ccb6a7d049d75a84071eb8ccd4cbab466fc1e55cc27791b560a4926f0c76` |
| `network-skill/scripts/windows_security.py` | 5444 字节；`efaf5591e9805b4319107cc890af26786bad690756077dbb8334ac632272b167` | 5594 字节；`23fa8c45076c4b25932f7ce28db02e3f0e74a06b69a0f30ed52f0222b8ae2bdd` |

Python 文件仅为换行差异：Release 有 150 个 LF、无 CRLF；CI 有 150 个 CRLF。换行归一后文本相同，与不同检出环境的换行转换相符。

主程序差异不限于 PE 时间戳：`.text` 虚拟大小分别为 7364720 / 7368304，`.rdata` 为 2940342 / 2937400；PE 时间戳分别为 `0x6acc8c40` / `0x6acc9e6d`。因此不能仅归因为安装器时间戳，也不能据此断定功能发生变化。

源码检查：`c2b7fa4` 到 `376bb7f` 只改动三个 Rust 文件中的测试代码，新增 Windows 所有者辅助函数有 `#[cfg(test)]`，CLI 测试模块有 `#[cfg(all(test, windows))]`；生产打包配置未改动。但公开 EXE 自身不能证明其实际源码提交。Release 声明的源码提交不等于二进制来源证明。

CI 日志确认 Windows Server 2025 runner，Bun 1.3.13，Rust 1.99.0 (`b940084d7`)，锁定前端依赖，命令为 `tauri build --bundles nsis --features custom-protocol`；之后执行 NSIS 守卫回归与 `verify-windows-package.py`。Windows 配置使用 `package-windows.py` 校验固定引擎并打包，工作流只上传 Artifact，没有 Release 发布步骤。

可能影响主程序的因素包括不同 Rust/MSVC/链接器版本、绝对构建路径、环境变量、检出换行和非确定性链接元数据。缺少原 Release 构建环境及命令记录，无法锁定主程序差异的唯一原因。应保留原包及其现有实机验收记录；后续从成功 CI 的同一份已验证 Artifact 提升为发布候选，避免重新构建，然后运行本校验。任何替换公开附件须另行明确授权。

## 可重复执行

本地仅需 Python 3（比较过程不运行安装包、不依赖 7-Zip）：

```sh
gh release download v0.2.0-windows.1 --repo wuxuebing-1/CCBaga --pattern CCBaga-A_0.2.0_x64-setup.exe --dir release
# 原始 ZIP 必须来自指定 Artifact ID；不要以 ZIP 摘要代替 EXE 摘要。
gh api -H "Accept: application/octet-stream" repos/wuxuebing-1/CCBaga/actions/artifacts/11737001917/zip > ci.zip
python scripts/compare-windows-release.py --release release/CCBaga-A_0.2.0_x64-setup.exe --artifact ci.zip --output comparison.json
```

`gh` 下载 Actions Artifact 需要具有 Actions 读取权限的登录账户。若 API 返回 JSON 而非 ZIP，比较脚本拒绝输入。
返回码：`0` 完全一致；`1` 不一致（仍生成 JSON）；`2` 输入错误。未来版本使用 `--asset-name` 指定精确 EXE 文件名。脚本复用现有验证器的流式 SHA-256，仅流式读取 ZIP 中唯一匹配 EXE，不解压归档指定路径。

在 Actions 中手动运行 **Windows release provenance**，填写 Release tag、成功 Windows run ID、Artifact ID 和 EXE 文件名。它验证运行成功、工作流路径、源仓库、标签提交、Artifact 所属运行/提交及未过期状态，下载精确附件并核对 GitHub 摘要，再比较内层 EXE；一致成功，不一致失败，保留元数据和 JSON 报告。权限仅 `contents: read` / `actions: read`，不发布、不安装。Artifact 过期后无法重新获取，需提前保留原 ZIP 与报告。

原 Windows 构建流程继续复用载荷验证器，并在成功验证后额外上传 `build-provenance.json`，记录源提交、run/attempt、工作流引用、EXE 大小和摘要。这是可追溯记录，不是密码学签名证明。既有历史包没有这份记录，本次从 API 元数据和实际下载字节核验。

验证：离线回归覆盖一致/不一致退出码、缺失或重复 EXE、文件名不匹配及路径穿越成员的安全读取；本次真实输入返回不一致。新增 Actions 工作流尚需合入后在线运行；未执行 Windows 实机安装测试。
