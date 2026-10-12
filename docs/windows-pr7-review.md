# PR #7 复用评估

审查时间：2026-10-12。对象：[liseami/CCBaga PR #7](https://github.com/liseami/CCBaga/pull/7)，
固定提交 `2a4f0a5aa68db3598566ea496b6354373a95c0f0`。本记录是源码静态审查，未执行该 PR 的清理或安装脚本。

结论：可参考其 Windows 平台拆分、指纹 Chromium 资源布局及减少子进程开销的思路，
但该提交不适合直接作为完整 Windows 成品替换本实现。查询时 PR 为 Open、未合并，GitHub 报告与主分支有冲突。
其声明的测试结果为作者提供的结果，本次没有在原 PR 分支复现。

| 发现 | 影响与证据 |
| --- | --- |
| Windows 检测仍按 Mac 判断 | `browser/network.rs` 非 Linux 分支要求平台包含 `mac`；真实 Win32 无法通过。前端也有相同条件。[固定源码](https://github.com/yq04/CCBaga/blob/2a4f0a5aa68db3598566ea496b6354373a95c0f0/src-tauri/src/browser/network.rs#L228-L239) |
| 整个 Desktop 目录成为删除目标 | `%APPDATA%/Claude`、`%LOCALAPPDATA%/Claude`、`AnthropicClaude` 被整体删除，可能移除 MCP 配置、扩展及经典版程序文件。[固定源码](https://github.com/yq04/CCBaga/blob/2a4f0a5aa68db3598566ea496b6354373a95c0f0/src-tauri/src/cleaner_targets.rs#L290-L304) |
| 安装后的 CLI 脚本路径缺失 | 从编译机源码路径或当前目录加载 `scripts/claude-cli.ps1`，Windows 安装资源未包含它；离开源码树后 CLI 管理会失败。[固定源码](https://github.com/yq04/CCBaga/blob/2a4f0a5aa68db3598566ea496b6354373a95c0f0/src-tauri/src/claude_cli.rs#L144-L156) |
| 卸载/重装错误未可靠传播 | 删除和包管理器错误被忽略；标准原生启动器未完整移除，旧版本可能使失败的重装看似成功。[固定源码](https://github.com/yq04/CCBaga/blob/2a4f0a5aa68db3598566ea496b6354373a95c0f0/scripts/claude-cli.ps1#L123-L182) |
| 进程识别和终止过宽 | 缺少当前用户 SID、精确安装路径和创建时间约束；直接按 PID/树强制终止。[固定源码](https://github.com/yq04/CCBaga/blob/2a4f0a5aa68db3598566ea496b6354373a95c0f0/src-tauri/src/windows.rs#L566-L623) |

此外，该 PR 仍修改旧版宿主内的 VPS 部署模块，而本次基础版本已把网络部署移到独立 Network Skill。
直接整体替换会回退这项架构调整。默认浏览器注册、HTTP/HTTPS 分别核验、正常退出浏览器、
最终安装载荷及实际指纹变化也需要补充验证。

本交付保留新版独立 Skill 架构，使用自己的精确清理白名单、进程/文件身份保护、内嵌 CLI 脚本、
双应用安装入口、Windows 判定和真实引擎测试。未把 PR 的未经修正实现直接合入。
后续可单独评估其原生命令行查询与 Bun 全局安装识别，并沿用相同安全回归后再集成。
