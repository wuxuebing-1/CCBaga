# scripts/
> L2 | 父级: ../CLAUDE.md
make-icon.swift: AppKit 将压缩品牌图绘入 macOS 圆角网格，保持原图并添加透明留白与投影。
make-icons.sh: 从 assets/ 品牌图生成各尺寸 iconset，iconutil 输出 src-tauri/icons 与 browser-runtime/icons。
brand-browser.sh: 编译 CCBrowser 入口、写入 CCBaga-B 名称/图标/各语言显示名、清除构建资源扩展属性并调用 sign-browser.py 完整签名（显式固定 ai.local.requirement-french.browser，干净上游引擎也必须与宿主一致）。
sign-browser.py: 内层 Mach-O → Helper 应用 → Framework → 浏览器外壳顺序签名；Developer ID 使用可信时间戳、Hardened Runtime，仅 Renderer/GPU 使用 JIT entitlement；本机临时签名清除 runtime，防止缺 Team ID 的 Framework 被库验证拒绝。
test-release-env.py: 伪造凭据环境导入真实发布入口，断言 Apple/更新私钥不进入构建子进程环境，不加载真实密钥。
test-sign-browser.py: 临时伪引擎树验证叶子到容器的签名顺序、最小 JIT 授权与符号链接不重复遍历。
fetch-browser.py: 固定上游引擎 DMG 与 SHA-256，核验版本和 arm64 后恢复引擎；干净 CI 检出不依赖维护者本机 Browser.app。
release-macos.py: 外部凭据与可选 CC_RELEASE_VERSION 驱动三阶段公证装订；明确将原生构建目录 Browser.app 暂存为交付名 CCBaga-B.app，再由 dmgbuild 无头写入 Finder 布局；装订后的宿主单独归档并签名绑定版本，更新私钥仅传给签名子进程；Apple 凭据不进入构建、DMG 或更新签名子进程，公证工具只通过明确参数接收其认证输入；官方更新器回归核验后生成公开 latest.json；产物和哈希记录写入被忽略的 releases/。
dmg-settings.py: 固定 800×500 像素背景、两应用到 Applications 的拖拽坐标；通知文件保留在盘内但默认隐藏，不依赖 Finder 自动化。
verify-dmg.py: 只读挂载实际安装盘，校验双应用名称、包标识与启动入口，再校验严格签名与 Gatekeeper、Applications 链接和像素背景；Finder 元数据不得破坏应用签名。
release-requirements.txt: dmgbuild、ds-store 与 mac-alias 的完整版本及 wheel SHA-256；仅发布阶段安装，不增大应用包。
package-browser.sh: 先品牌化签名，再打包引擎，ditto 保留 Framework 链接与可执行权限；归档进入应用资源；Linux 将源引擎复制到 CARGO_TARGET_DIR 的 release 资源树，保持源码/Arch 安装契约，系统包另走 libexec。
dev-install.py: 本机 Apple Silicon 快速 debug/custom-protocol 构建、临时签名与双应用重装，安装前验证 Chromium 实际 CLI 可加载；过滤 Apple/更新私钥环境；不公证、不生成 DMG、不发布。
test-install-apps.py: 安装模式信任边界与环境隔离的离线回归，不操作真实应用或密钥。
install-apps.sh: bun install:apps 兼容入口，只转交统一安装层。
install-apps.py: 正式包严格签名/Gatekeeper；--development 仅本机 debug/临时签名，拒绝外部 DMG；验收后的事务替换、同标识旧名迁移与失败回滚；保留隔离和资料，正常退出后安装、只刷新相关 Dock 项并启动宿主。
claude-cli.sh: 原生宿主内嵌的只读安装检查与官方 CLI 卸载/重装，使用登录终端 PATH，发现 nvm/全局 npm、Homebrew 双通道与原生版本；只卸载程序，保护自定义启动器与配置资产，重装沿用已识别包管理器并核验版本。
install-linux.py: Linux 用户前缀安装并保留资料；拒绝自定义启动器、链接父目录与整个安装目录下仍运行的 A/B，失败回滚。
test-linux.py: 临时构建验证安装隔离、B 运行保护、桌面写入回滚、Exec 引用与载荷缺失/引擎篡改拒绝；不访问用户资料。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

release-linux.py: 无 Apple/更新凭据的 x86_64 三格式构建，先运行 Skill 离线回归与脚本语法检查；Tauri 以完整工具日志交付 DEB/AppImage，标准 rpmbuild 从相同 /usr 文件树生成 RPM，避免上游 RPM 打包停滞且禁止剥离厂商引擎；引擎位于 libexec，避免 Qt 可选适配库被宿主 linuxdeploy 错当必备依赖；真实解包核对宿主、UI、B 入口、图标与引擎字节，生成独立校验清单。
test-rpm.py: 临时 DEB 文件树经真实 rpmbuild 转换，回归 RPM 版本/架构、桌面入口与引擎字节，不执行样本程序。
verify-linux-package.py: Linux 安装包只读载荷契约，供 Ubuntu/Arch 共用，拒绝缺失 UI、入口或被改写的引擎文件/链接。
build-arch.sh: 固定摘要容器准备 Node.js 等构建/测试依赖，以非 root makepkg、原生回归与解包验收，公开源码与 Cargo 缓存挂载；不接触 macOS 凭据。

package-windows.py：构建前核验固定 Windows 引擎完整清单、PE/版本与双 UI 资源。verify-windows-package.py：7-Zip 只读解开真实 NSIS，比对宿主、完整引擎、UI 和许可并生成 SHA256SUMS。test-windows-browser.py/smoke-windows-browser.py：恶意归档/PE 回归与真实隔离浏览器种子/平台观測。claude-cli.ps1/test-claude-cli.ps1：Windows 原生/npm/WinGet 程序管理和隔离测试。

test-windows-installer.py：实际编译并运行 NSIS 安装/卸载守卫（1024 字符限制），验证完整脚本、64 位 PowerShell、首装/占用/相邻进程以及中文、空格和单引号路径；可丢弃进程保持运行。

Windows 安装验收同时按源文件字节核验随包 Windows 网络 Skill；资源清单排除 Python 临时缓存。

compare-windows-release.py/test-windows-release-comparison.py：复用现有摘要函数，比较 Release EXE 与原始 CI ZIP 内唯一安装包，生成 JSON 与明确退出码；离线回归不执行程序。verify-windows-package.py 成功后记录 build-provenance.json。
