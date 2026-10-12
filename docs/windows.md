<!-- [INPUT]: Windows community prerelease, upstream architecture and actual validation evidence.
[OUTPUT]: Canonical installer/source downloads, Windows usage/build guide and verification boundaries.
[POS]: Community platform guide; original MIT license, upstream credits and release authority remain intact. -->
# Windows x64（社区预览版）

## 下载与快速开始

版本：[`v0.2.0-windows.1`](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1)，Windows 10/11 x64 社区预发布。

- [下载 CCBaga-A_0.2.0_x64-setup.exe](https://github.com/wuxuebing-1/CCBaga/releases/download/v0.2.0-windows.1/CCBaga-A_0.2.0_x64-setup.exe)：唯一安装包，内含 A、B 的原版指纹浏览器引擎和 Windows 兼容 Network Skill。
- [下载对应源码 ZIP](https://github.com/wuxuebing-1/CCBaga/archive/refs/tags/v0.2.0-windows.1.zip)：包含原有 macOS/Linux 实现及 Windows 移植；构建时另行下载摘要固定的浏览器引擎。
- [发布说明与校验文件](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1) · [中文验收记录](windows-validation.md)。

运行安装包后，从开始菜单打开 **CCBaga-A** 进行本地清理、设置与核验，它会带起使用独立资料的 **CCBaga-B**。B 也有单独的开始菜单入口。先阅读清理预览再确认删除；选择地区不会改变真实公网 IP，绿色结果不代表匿名保证或所有泄漏检查已通过。

修正版已在 Windows 11 完成实际安装，并核对安装文件和 A/B 快捷方式。干净 Windows 10 的完整安装/卸载、真实 CLI 重装、管理员设置和 VPS/TUN 仍需在目标环境验收。当前安装包未签名；Windows 后续升级需手动下载完整安装包，先退出 A 和该安装的 B，不走 macOS 签名自动更新通道。

This fork adds a native Windows 10/11 x64 port of [CCBaga by liseami](https://github.com/liseami/CCBaga). It extends upstream
commit `addd0a67d34fa95da2bccf1026a44b1b3ec78304`; it is a community development
prerelease, not an official release by the upstream maintainer. The existing macOS and
Linux implementations and [MIT license](../LICENSE) remain in the same source tree;
third-party components retain the terms in [NOTICE.md](../NOTICE.md).

The original objective is unchanged: local maintenance, an isolated fingerprint
browser, and **measurement of the resulting environment**. Selecting a region
does not change the public IP. A green verification result still requires real
browser measurements, a received loopback report, and matching machine/network
state. DNS/WebRTC checks remain explicitly unverified.

## Installation

Use [CCBaga-A_0.2.0_x64-setup.exe](https://github.com/wuxuebing-1/CCBaga/releases/download/v0.2.0-windows.1/CCBaga-A_0.2.0_x64-setup.exe) from the [community prerelease](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1). It installs for
the current user and includes the genuine pinned Fingerprint Chromium engine;
there is no separate browser download. The installer also includes the Windows-compatible Network Skill; the Network page copies an installation prompt referencing that local copy, so it does not silently fetch an older unported Skill. Microsoft Edge WebView2 is required for
the host interface; the installer offers the official runtime bootstrapper when
it is absent. Installation may therefore require internet access.

Start **CCBaga-A** for maintenance. The **CCBaga-B** Start-menu shortcut launches
the same host with `--browser`, which opens its isolated companion browser and
establishes a fresh report session. A also starts B automatically. B's profile is
under `%APPDATA%\ai.local.requirement-french\browser\profile`, separate from Edge,
Chrome and other Chromium profiles. Closing A's window keeps its tray process;
use the tray's Quit command to exit it.

The community installer is unsigned. Signed automatic updates currently belong
to the upstream macOS distribution; Windows upgrades are manual, using a complete replacement installer from this fork's Releases. Exit A through its tray menu and close B before upgrading.
Uninstalling the program preserves browser profiles, settings and Claude data.
Upgrade/uninstall refuses to proceed while this installation's B browser is still
running. It does not stop unrelated Chrome processes.
No self-signed certificate is installed and no Windows protection is disabled.

## Feature preservation

| Core capability | Windows implementation |
| --- | --- |
| Claude cleanup | Existing snapshot/allowlist/confirmation transaction, default `.claude` data, roaming Desktop and exact MSIX package storage, local caches, selected history/backups and narrow configuration edits |
| Protect personal assets | Keeps projects, skills, plugins, hooks, MCP configuration and `claude_desktop_config.json`; does not delete the entire Desktop directory |
| File safety | Current-user ownership, volume/file identity, metadata snapshots, junction/reparse-point rejection including ancestors, ACL-preserving atomic config replacement |
| Claude process control | Current-user SID plus executable and creation-time identity; native/npm/WinGet CLI, recognized Desktop install and exact current-user MSIX package |
| Credentials | Allowlisted Windows Credential Manager item metadata; password blobs are never inspected; deletion revalidates the item reference |
| CLI tools | Native, npm and exact WinGet package detection/uninstall/reinstall with progress, Job Object timeout cleanup and installation verification |
| Browser | Genuine Fingerprint Chromium 148.0.7778.215 Windows x64, pinned archive digest, independent profile, seeded rotation, controlled shutdown and real reporting |
| System settings | Windows IANA/timezone mapping, preferred language/region changes, actual reread after changes; timezone elevation only if required |
| Location and default browser | Windows Settings and current-state verification; never silently changes a protected user choice |
| Network Skill | Existing separate SSH/deployment workflow with Windows private DACLs, OpenSSH and process-tree timeouts; remote Debian requirement and approval steps remain |
| Localization | Shared English/Chinese UI; UI language remains independent of machine/browser locale |

Windows language packs, organization policy or a required sign-out can prevent a
setting from taking effect. Such operations must be checked in the real user
environment; opening Settings is not reported as a successful setting change.
Unknown location state stays unknown. No bypass of Windows location policy or
default-app confirmation is attempted.

Default cleanup covers recognized native Windows locations. A custom
`CLAUDE_CONFIG_DIR`, redirected/custom shell profiles, WSL distributions and
cloud-side data are not silently assumed to have been cleaned. PowerShell profile
cleanup only transforms supported static single-line environment assignments;
compound/dynamic commands are preserved for manual review. Read the actual scan
before confirming deletion. Selected data is permanently deleted without backup
or undo, as in the original project.

## Build from source

Start from the [source ZIP for `v0.2.0-windows.1`](https://github.com/wuxuebing-1/CCBaga/archive/refs/tags/v0.2.0-windows.1.zip) or check out that tag in this repository. Source archives do not contain the external Chromium binaries or build dependencies.

Install stable Rust with the MSVC target, Visual Studio C++ Build Tools with a
Windows SDK, Bun, Python 3.10+ and WebView2. A Windows 11 x64 machine with Rust
1.99.0 and Bun 1.3.13 was used for local development. 7-Zip is needed to inspect
the final installer payload. Follow [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)
for the native tools.

```powershell
bun install --frozen-lockfile
python scripts/fetch-browser.py
bun run build:windows
python scripts/verify-windows-package.py src-tauri/target/release/bundle/nsis
```

The installer and `SHA256SUMS.txt` are in `src-tauri/target/release/bundle/nsis`.
`fetch-browser.py` refuses to overwrite an existing prepared engine. It validates
the publisher's pinned archive hash, x64 PE architecture, exact version and safe
archive paths, then records all extracted engine files. Build preflight rechecks
that manifest. Never replace this engine with stock Chrome: that loses seeded
fingerprint behavior.

For development after fetching the engine:

```powershell
bun run dev:windows
```

Tauri automatically merges `tauri.windows.conf.json`; Windows does not execute the
macOS shell packaging script. Windows CI builds an artifact with read-only GitHub
permissions and does not impersonate the upstream official release workflow.

## Validation

```powershell
bun test
bun run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
python scripts/test-windows-browser.py
python scripts/test-windows-installer.py
powershell -NoProfile -File scripts/test-claude-cli.ps1
python skills/ccbaga-network/scripts/test_network.py
python scripts/smoke-windows-browser.py
```

The [validation record](windows-validation.md) distinguishes automated checks from the completed Windows 11 installation and the remaining target-environment checks. Tests use disposable files/processes and local HTTP servers. The genuine-engine
smoke test confirms actual platform/version/CPU/language/timezone readings and
Canvas stability for one seed versus a changed seed. It does not use personal
browser data or change Windows settings. Permission-dependent system changes,
real CLI uninstall/reinstall, real VPS/TUN setup, Windows default-app selection,
installation on a clean Windows 10 machine and signed distribution still require
separate end-to-end acceptance. Passing unit/smoke tests does not claim those
operations were performed on the developer's computer.
