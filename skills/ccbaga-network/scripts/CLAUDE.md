# scripts/
> L2 | 父级: ../CLAUDE.md
network.py: AI 标准输入配置、阶段式密钥/主机/出口验证与显式部署，无用户终端交互；验证后写入 Git/Skill 外的私密本机配置及临时私钥（Unix 0600、Windows 当前用户 ACL）、严格后续 SSH、拒绝目录联接、有界脱敏输出；订阅经 SSH 取本机私密文件，curl TUN 实测不使用代理环境。
windows_security.py: 标准库 ctypes 调用 Windows API，在文件创建时设置当前用户私密 DACL，读取前校验所有者/允许项；Job Object 在任务超时或退出时结束整棵本次子进程树，不依赖 taskkill 竞争窗口。
setup-vps.sh: 原部署脚本迁出宿主；仅 Debian 12/13 root，系统/SSH/防火墙与 Xray/Reality 套 SOCKS5，UDP 阻断；自检全部通过才生成订阅回报，身份保存在 root 状态文件。
test_network.py: 可丢弃密钥/文件和本机子进程，跨平台验证三文件配对、Unix 权限/Windows ACL、地址/注入拒绝、非交互配置拒绝越界/覆盖、输入传输和含后代的超时；无 Windows 符号链接权限时明确跳过该单项；不连接真实 VPS，不使用用户凭据。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
