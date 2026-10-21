# Windows 移植验收记录

日期：2026-10-12。基础版本：`addd0a67d34fa95da2bccf1026a44b1b3ec78304`。
本机验证环境：Windows 11 x64（10.0.26200）、Rust 1.99.0、Bun 1.3.13。

发布标识：[`v0.2.0-windows.1` 社区预发布](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1)。
安装包：[CCBaga-A_0.2.0_x64-setup.exe](https://github.com/wuxuebing-1/CCBaga/releases/download/v0.2.0-windows.1/CCBaga-A_0.2.0_x64-setup.exe)。
对应源码：[固定发布标签 ZIP](https://github.com/wuxuebing-1/CCBaga/archive/refs/tags/v0.2.0-windows.1.zip)。

本记录对应已修复占用检查的安装包，其 SHA-256 为：

```text
c7a4b7df48d28e706f77a36f0fdf3c4d584561fa1aac27504f9c80079e8966c3
```

原项目的目标继续保持为：本地清理、独立指纹浏览器、系统设置与实际环境核验。
本移植保留 A/B 协作与回环报告；地区选择不伪造公网出口，未知检测项不能判为通过。
原 macOS/Linux 平台实现和 MIT/第三方许可保留。

| 验证 | 结果 |
| --- | --- |
| TypeScript 类型检查、Vite 生产构建 | 通过 |
| 前端单元测试 | 16 通过，0 失败 |
| Windows Rust 测试 | 45 通过，0 失败；原有官方签名发布包专用测试 1 项按上游规则忽略 |
| Windows 引擎 ZIP/PE 验证测试 | 3 通过 |
| NSIS 安装/卸载占用守卫 | 实际可丢弃进程测试通过；本安装目录阻止、相邻目录放行，不终止进程 |
| PowerShell CLI 管理测试 | 隔离测试通过；没有卸载或重装用户的实际 CLI |
| 网络 Skill | 8 通过；1 项需创建符号链接的测试因 Windows 权限跳过；另有原生 junction 安全测试通过 |
| 真实 Windows Fingerprint Chromium | 3 次独立临时资料运行通过：Win32、10.0.26200、20 核、en-US、America/New_York；同种子 Canvas 稳定、换种子改变 |
| NSIS 最终安装包 | 通过：只读解包后核对主程序（仅允许 Tauri 的 NSIS 标记变化）、完整浏览器、双入口页面及全部资源、Windows 网络 Skill 与许可文件 |

清理测试覆盖：过期快照、同大小文件改写并恢复 LastWriteTime、祖先目录联接、配置原子替换、
只停止捕获到的 CLI 进程、MCP/技能/扩展/项目保护、MSIX 精确包路径与相邻包保护。
安装包包含原版指纹引擎及 Windows 兼容网络 Skill；网络页复制的是随包 Skill 的本机来源。

尚未执行的真实环境验收：干净 Windows 10 的完整安装/卸载、真实 Claude CLI 卸载重装、
管理员授权系统变更、显示语言包/注销效果、用户默认浏览器确认，以及真实 VPS/家宽/TUN。
构建未进行 Authenticode 签名，属于本仓库的社区预发布，不是上游官方 Windows 版本。未把这些待验收项记作测试成功。Windows 使用手动下载安装包升级，不使用上游 macOS 签名自动更新。

源码构建、能力对照与复现命令见 [Windows 说明](windows.md)。

## 安装器占用检查修复与实际安装验证

用户实际安装暴露了原验证未覆盖的问题：旧版把 2,976 字符 Base64 脚本放入 NSIS 命令字符串，
超过 NSIS 1,024 字符限制。安装器把它截为 1,023 字符，PowerShell 返回无效编码错误；
这不是用户没有关闭浏览器，也不能用直接运行 PowerShell 成功来证明安装器执行成功。

修复改为内嵌完整脚本文件，通过短 `-File` 命令调用原生 64 位 PowerShell；
确认引擎目录不存在时按首装处理，错误提示支持中文并提供错误码。
真实编译的 NSIS 安装/卸载回归已通过：首装、目录边界、占用阻止、退出后放行、
中文/空格/单引号路径、进程不被终止、1024 字符环境和实际 PowerShell 位数。

完整安装验证：上述修正版 NSIS 已在用户选择的原目录完成实际安装，退出码 `0`。
逐项比对安装后的主程序、完整浏览器、页面和网络 Skill 共 96 个文件，并检查卸载程序/许可文件。
开始菜单 `CCBaga-A` 指向宿主，`CCBaga-B` 指向同一宿主且带 `--browser`；两个快捷方式均通过核验。
本项是 Windows 11 的实际安装验证；未扩大为 Windows 10 或真实清理操作的验收结论。

## 发布来源补充核验

公开包与成功 CI 包的实际 SHA-256 不一致；具体载荷差异与复验方法见 [发布来源记录](windows-release-provenance.md)。本页原实机验收针对当时的公开包，不能转移为 CI EXE 的实机安装结论。
