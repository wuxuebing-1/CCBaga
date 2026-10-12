<!--
[INPUT]: 已实现功能、Windows 社区预览版、双应用使用方式与 MIT/第三方许可边界。
[OUTPUT]: Windows 中文快速开始、原项目功能说明、各平台安装开发入口与贡献导航。
[POS]: 社区分支公开入口；保留上游署名，区分本仓库 Windows 发布与上游官方版本。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
<div align="center">
  <img src="docs/brand-logos.svg" width="320" height="149" alt="CCBaga-A and CCBaga-B logos, side by side" />
  <h1>CCBaga</h1>
  <p><strong>A native macOS, Linux and Windows toolkit for a cleaner workspace and an environment you can verify.</strong></p>
  <p>Claude cleanup · Browser fingerprints · System settings · Network Skill</p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-e8a181?style=flat-square" alt="MIT license" /></a>
    <img src="https://img.shields.io/badge/macOS-12%2B-111827?style=flat-square&amp;logo=apple&amp;logoColor=white" alt="macOS 12 and later" />
    <img src="https://img.shields.io/badge/Linux-x86__64-111827?style=flat-square&amp;logo=linux&amp;logoColor=white" alt="Linux x86_64" />
    <img src="https://img.shields.io/badge/Windows-10%2F11%20x64-111827?style=flat-square&amp;logo=windows&amp;logoColor=white" alt="Windows 10/11 x64 community port" />
    <img src="https://img.shields.io/badge/core-Rust%20%2B%20Tauri-111827?style=flat-square&amp;logo=rust&amp;logoColor=white" alt="Rust and Tauri" />
  </p>
  <p>
    <a href="#features">Features</a> ·
    <a href="#installation">Installation</a> ·
    <a href="#updates">Updates</a> ·
    <a href="#quick-start">Quick start</a> ·
    <a href="#architecture">Architecture</a> ·
    <a href="#development">Development</a> ·
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>
</div>

![CCBaga: native control, observable environments](docs/readme-hero.png)

## Windows 快速开始（社区预览版）

本仓库基于 [liseami/CCBaga](https://github.com/liseami/CCBaga) 增加 Windows 原生支持，保留上游的 macOS/Linux 源码、MIT 授权与第三方许可。核心目标仍是**本地白名单清理、独立指纹浏览器、系统设置与实际环境核验**。这是 `wuxuebing-1/CCBaga` 的社区移植，不是上游维护者发布的官方 Windows 版本。

**下载 [Windows x64 安装包](https://github.com/wuxuebing-1/CCBaga/releases/download/v0.2.0-windows.1/CCBaga-A_0.2.0_x64-setup.exe)** · [预发布说明与校验文件](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1) · [对应源码 ZIP](https://github.com/wuxuebing-1/CCBaga/archive/refs/tags/v0.2.0-windows.1.zip)

1. 运行 `CCBaga-A_0.2.0_x64-setup.exe`。一个安装包包含 A、原版指纹浏览器引擎和 Windows 兼容 Network Skill；无需另装 B。缺少 WebView2 时，安装器会下载微软运行时。
2. 从开始菜单启动 **CCBaga-A**，进行清理、设置和核验；它会带起 **CCBaga-B**。B 使用独立浏览器资料，也可从自己的开始菜单入口打开。
3. 先查看扫描或检测结果，再确认操作。选择地区不会改变公网 IP；绿色核验只代表已测项目匹配，**不保证匿名、账号平台接受或 DNS/WebRTC 无泄漏**。

当前为未签名的 `v0.2.0-windows.1` 预发布，面向 Windows 10/11 x64。已验证 Windows 11 的实际安装、完整安装文件和 A/B 快捷方式；干净 Windows 10 的安装/卸载及真实 CLI 重装、管理员设置和 VPS/TUN 仍待目标环境验收。Windows 升级需手动下载完整安装包，先退出 A 和本次安装的 B；不使用 macOS 的签名自动更新通道。

[Windows 使用与构建说明](docs/windows.md) · [完整验收记录与限制](docs/windows-validation.md) · [PR #7 复用评估](docs/windows-pr7-review.md)

CCBaga brings local maintenance and environment setup into one focused desktop workflow. Inspect Claude data before removing it, rotate your companion browser's fingerprint and restart it, align system settings with a selected region, and compare real browser readings with the machine and network exit.

Two applications work together: **CCBaga-A** owns native operations; **CCBaga-B** runs an isolated Fingerprint Chromium profile. Browser measurements return to the cleaner through a local session. Changing a setting and verifying its effect are separate steps.

## Features

<table>
  <tr>
    <td width="33%"><strong>Local Claude cleanup</strong><br /><br />Review caches, account traces, sessions, configuration remnants and existing backups. Select exactly what to remove.</td>
    <td width="33%"><strong>Fingerprint &amp; restart</strong><br /><br />Generate a fresh seed, apply the selected region's language and timezone, then restart the companion browser.</td>
    <td width="33%"><strong>Real environment checks</strong><br /><br />Compare browser and machine readings with the actual network exit. Passing measured checks produces a green page and a large checkmark.</td>
  </tr>
  <tr>
    <td><strong>Native system settings</strong><br /><br />Apply timezone, preferred language and region settings. Open desktop controls for location and permissions; see the platform guides for limits.</td>
    <td><strong>AI-guided network setup</strong><br /><br />Three simple steps, a coding-agent selector and a copyable Skill installation prompt. Give the AI your files and connection details; it handles the commands.</td>
    <td><strong>Claude Code CLI tools</strong><br /><br />Detect the installation method and version. Uninstall or reinstall through native, npm, Homebrew or Windows WinGet workflows, according to the platform.</td>
  </tr>
</table>

**One region selector. Four everyday actions.** Easy setup puts browser cache cleanup, fingerprint rotation, timezone/language reset and Claude cleanup on the same page. The sidebar keeps advanced controls in Network, Claude cleanup, Browser and Computer.

The interface supports **English and Chinese**, with shared YAML catalogs. Switching the interface language does not change the computer or browser language.

## Installation

**Platforms:** macOS 12 or later on Apple Silicon; Linux x86_64 with GTK 3 and WebKitGTK 4.1. This checkout adds Windows 10/11 x64 support; see [Windows setup and validation](docs/windows.md). Intel macOS and Linux ARM64 are not supported.

Linux x86_64 packages include DEB, RPM, AppImage and Arch/pacman formats, with the companion browser included. See [Linux setup](docs/linux.md) for installation and platform limits. Official versions are published exclusively by the project owner, **liseami**.

### Windows (community port)

Download [CCBaga-A_0.2.0_x64-setup.exe](https://github.com/wuxuebing-1/CCBaga/releases/download/v0.2.0-windows.1/CCBaga-A_0.2.0_x64-setup.exe) from this fork's [Windows prerelease](https://github.com/wuxuebing-1/CCBaga/releases/tag/v0.2.0-windows.1). It includes the host, genuine pinned fingerprint browser and Windows-compatible Network Skill. See [Windows setup](docs/windows.md) for the tested scope and source-build instructions. This unsigned community release uses manual upgrades with complete installers.

### macOS

Download the official installer from [GitHub Releases](https://github.com/liseami/CCBaga/releases/latest). The distribution package is named `CCBaga-<version>-macos-arm64.dmg`. It contains both applications:

| Application | Installed bundle | Role |
| --- | --- | --- |
| CCBaga-A | `CCBaga-A.app` | Cleanup, system settings and environment verification |
| CCBaga-B | `CCBaga-B.app` | Companion browser with a dedicated profile |

Download **one DMG**; it contains both applications. There is no separate browser download. The installer uses the same pixel-art style as this README.

1. Open the disk image and drag **both applications** into Applications.
2. Launch CCBaga-A. It starts the companion browser automatically.
3. Choose English at the bottom of the sidebar if needed.

Source builds are available through the [development guide](#development).

## Updates

**Windows:** download a complete replacement installer from [this fork's Releases](https://github.com/wuxuebing-1/CCBaga/releases). Exit A through its tray menu and close this installation's B browser before upgrading. Windows does not use the upstream macOS automatic updater.

**macOS:** use **Check for updates** at the bottom of CCBaga-A’s sidebar. Startup checks quietly; installation only starts when you choose **Update and restart**. Downloads show actual progress and are verified before installation. Failed downloads can be retried; a failed restart does not download the update again.

macOS in-app updates replace **CCBaga-A**. To update **both A and B**, download the latest upstream DMG and drag both apps into Applications.

## Quick start

### Everyday setup

Open **Easy setup**, select a country or region, then choose an action. Browser fingerprint rotation closes and restarts CCBaga-B, so save unfinished browser work first. Selecting a region changes language, timezone and regional settings; it does **not** change your public IP.

### Claude cleanup

Open **Claude cleanup** to scan and review local data. Use cache-only cleanup for regenerable files, login reset for account traces, or select a broader scope explicitly. A folder can also be dropped into the app for a scoped scan.

If Claude is running, the confirmation flow can stop all recognized Claude processes and rescan. The latest scan must be reviewed before deletion. CLI uninstall and reinstall are separate operations and preserve settings, skills, sessions and project files.

> [!WARNING]
> Confirmed cleanup permanently deletes the selected files, including selected existing backups. It does not use Trash, create recovery copies or provide undo. Stopping Claude processes can interrupt unsaved work.

### Browser verification

Open **Browser** or the companion browser's local test page to measure the actual environment. Checks cover the network exit, country, timezone, language, platform, CPU count, engine version and location settings.

The success page appears only when the measured checks match, the report reaches the cleaner and no measurement error remains. **DNS and WebRTC leak checks are not independently verified.** A green result describes the measured checks, not a guarantee of anonymity or account acceptance.

### Network setup

Open **Network** for three steps: **Get VPS → Get residential proxy → Install Skill**. Select **General**, **Codex**, **Cursor**, **OpenCode** or **Copilot**, then copy the installation prompt into your local coding agent. The prompt follows the app's English/Chinese setting.

The [Network Skill](skills/ccbaga-network/README.md#installation) installs separately from the apps. Give the AI your VPS address, the three matching PEM/PPK/PUB files or downloaded key bundle, and your residential SOCKS5 connection details. It handles file paths, private configuration, commands and checks; you do not need to open a terminal or edit JSON. Missing resources are purchased by you with the AI's guidance. Remote setup runs only after approval of the actual server changes.

The Skill supports authorized Debian 12/13 servers using root key login on port 22. It checks the SSH identity and fixed SOCKS5 exit, configures Xray/Reality and delivers a private local import file after server self-checks pass. Client setup uses **TUN ON**, **System Proxy OFF** and an empty explicit browser proxy; actual TUN/browser verification remains a separate step. The subscription URL uses HTTP and contains credentials; prefer the file retrieved over SSH.

Provider buttons link to [DMIT](https://www.dmit.io/aff.php?aff=28800) and [Webshare](https://www.webshare.io/?referral_code=qlm1mt9xibc9). Network setup is optional; local cleanup does not require a VPS or a proxy subscription. **The app does not collect SSH keys or proxy credentials and does not deploy to servers.**

## Architecture

![Pixel architecture map: native core, companion browser and a separate AI Network Skill](docs/architecture.svg)

The UI owns inputs and presentation. Rust owns validation, task locks, process identity checks and execution. Small Objective-C bridges call macOS APIs; Rust Linux/Windows boundaries use native process identity, filesystem and desktop/session APIs. The browser test page has no Tauri command access; it exchanges context and measurements through a random-token loopback session.

| Layer | Technology | Responsibility |
| --- | --- | --- |
| Desktop host | Rust · Tauri 2 | Native commands, transactions, progress and lifecycle |
| Interface | React 19 · TypeScript · Cromma UI | Forms, previews and execution results |
| Platform bridges | AppKit · Security · CoreLocation · Linux Rust · Win32 | Platform permissions and system integration |
| Companion browser | Fingerprint Chromium | Dedicated profile and real browser measurements |
| Separate Network Skill | Python standard library · OpenSSH · Bash · Xray | AI-managed private configuration, approved remote setup and server/client verification |
| Localization | YAML · Vite | Build-time English/Chinese catalogs |

<details>
<summary><strong>Repository map</strong></summary>

```text
assets/             Compressed brand logos and pixel-art DMG background
src/                Desktop UI, browser test page and localization
src-tauri/src/      Rust commands and transaction boundaries
src-tauri/native/   Small macOS API bridges
browser-runtime/    Engine metadata, launcher, entitlements and licenses
scripts/            Icons, local builds and installation helpers
skills/             Installable AI Network Skill and its execution scripts
packaging/arch/     Native Arch Linux PKGBUILD and desktop entry
packaging/windows/  Windows installer hooks and browser occupancy checks
docs/               Platform guides, verification records and README artwork
vendor/cromma-ui/   Vendored interface components
```

The large browser application and archive are generated/runtime inputs, excluded from Git. `CLAUDE.md` files map module boundaries; source headers describe inputs, outputs and responsibilities.

</details>

## Development

### Prerequisites

On Windows, follow [Windows setup](docs/windows.md). On Linux, follow [Linux setup](docs/linux.md). On macOS, use an Apple Silicon Mac with Xcode Command Line Tools, stable Rust and Bun. Follow [Tauri's macOS prerequisites](https://v2.tauri.app/start/prerequisites/#macos) for the native toolchain.

The browser engine is an external build input. `scripts/fetch-browser.py` downloads the pinned **Fingerprint Chromium 148.0.7778.215** engine for the supported host platform from the [upstream project](https://github.com/adryfish/fingerprint-chromium), verifies its SHA-256 and preserves its structure and notices. Browser runtime binaries are not included in the source repository. Use [the tagged source ZIP](https://github.com/wuxuebing-1/CCBaga/archive/refs/tags/v0.2.0-windows.1.zip) for this Windows prerelease and follow the [Windows build commands](docs/windows.md#build-from-source).

For a macOS checkout:

```sh
bun install --frozen-lockfile
python3 scripts/fetch-browser.py
sh scripts/make-icons.sh
sh scripts/package-browser.sh
bun run tauri dev
```

Local builds are for development and testing; they are not official project downloads.

### Fast local build and install (macOS)

```sh
bun run dev:install
```

This builds a debug app with the embedded UI, uses local ad-hoc signatures and installs/starts both apps. It does not use Apple credentials or create an installer. It preserves profiles and settings. The development installer accepts only locally built debug apps; downloaded installers still require the normal signature and Gatekeeper checks.

### Build

```sh
MACOSX_DEPLOYMENT_TARGET=12.0 \
  bun run tauri build --bundles app --features custom-protocol
```

The cleaner bundle is written to `src-tauri/target/release/bundle/macos/CCBaga-A.app`. The companion bundle remains at `browser-runtime/Browser.app` until copied to Applications as `CCBaga-B.app`.

Production builds require `custom-protocol` to load the embedded UI. Compilation without it fails explicitly, preventing an installed app from depending on the development server.

### Checks

```sh
bun run build
bun test
MACOSX_DEPLOYMENT_TARGET=12.0 \
  cargo test --manifest-path src-tauri/Cargo.toml
python3 scripts/test-sign-browser.py
python3 skills/ccbaga-network/scripts/test_network.py
bash -n skills/ccbaga-network/scripts/setup-vps.sh
```

Automated tests exercise disposable fixtures and local test servers. They do not deploy a real VPS, delete your Claude account data or perform administrator-authorized system changes. Real SSH connectivity, client-side TUN routing and permission-dependent operations require validation in the target environment.

## Data & permissions

- **Cleanup is local.** It targets recognized Claude data; project source, skills, plugins, hooks, commands and MCP settings are preserved. Cloud-side account records are outside its scope.
- **Secrets stay out of UI logs.** The apps have no SSH key picker or server deployment command. Credential cleanup reads item attributes, not password contents. The separate Skill uses private local files and temporary PEM copies: mode `0600` on Unix, a restricted current-user ACL on Windows. Data supplied to your coding agent is subject to that agent provider’s handling.
- **Network checks make real requests.** Exit detection uses `ipwho.is` with `ipapi.co` as a fallback on the same route. IP location is a service estimate.
- **Remote changes are explicit.** The separate Skill contacts only the VPS you provide after server-change approval. Required credentials remain in private local/server configuration, outside Git and public logs.
- **Permissions are feature-specific.** Timezone changes require macOS administrator authorization. Keychain access follows system prompts. Location controls may require Accessibility permission or a manual action in System Settings.

Custom `CLAUDE_CONFIG_DIR` locations are not automatically included in the default cleanup scan. The app reports this limitation rather than silently claiming they were cleaned.

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Small, focused changes with clear behavior and relevant checks are welcome. For bug reports, include the operating system and version, app version, operation and sanitized error; never attach private keys, passwords or subscription URLs.

## License & credits

CCBaga's original project code is released under the **[MIT License](LICENSE)**. Third-party code and binaries retain their own terms; see [NOTICE.md](NOTICE.md).

This Windows community fork builds on [CCBaga by liseami](https://github.com/liseami/CCBaga). Upstream authorship, licenses and the existing macOS/Linux implementations are retained. Releases from this repository are distinct from the upstream maintainer's official releases.

Built with [Tauri](https://tauri.app/), [React](https://react.dev/) and [Fingerprint Chromium](https://github.com/adryfish/fingerprint-chromium), with a native macOS implementation of Claude maintenance workflows. Cleanup discovery was informed by the [claude-cleanup skill](https://github.com/suyuan2022/suyuan-skill/tree/main/claude-cleanup).

<div align="center">
  <sub>An independent project. Not affiliated with or endorsed by Anthropic or Apple.</sub>
</div>
