# .github/workflows/
> L2 | 父级: ../CLAUDE.md
release.yml: 仅 liseami 在 liseami/CCBaga 的 release push/手动触发及重跑可发布；统一版本后并行 macOS、Ubuntu x86_64 和固定摘要 Arch；Linux 无签名凭据且解包核验 DEB/RPM/AppImage/pacman 后才允许发布；arm64 runner 哈希安装 DMG 工具、缓存 Rust 依赖（版本号仅构建步骤注入，不进入依赖缓存键）、测试前端/原生/Skill/开发安装边界 并签名公证，临时钥匙串持有 Secrets，固定 SHA Actions；独立最小权限发布任务读取不可变产物，校验实际签名更新包并交付 latest.json，成功后显式固定并核对递增版本 tag，用当前产品与安装说明发布 Release（不生成历史比较列表），避免分支推进后隐式建 tag 的权限失败，公开仓库附带来源证明。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

windows.yml：Windows PR/push/手动测试与 NSIS 开发产物，GitHub 只读权限、不使用签名密钥、不自动发布 Release；原 release.yml 的官方权限不变。
