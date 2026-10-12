# packaging/
> L2 | 父级: ../CLAUDE.md
arch/ - 社区 Arch x86_64 构建配方与桌面入口，不拥有官方发布权限；见其 CLAUDE.md。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

windows/hooks.nsh：当前用户开始菜单 CCBaga-B 入口与只删除本安装注册关联的卸载钩子；不触碰浏览器资料或 UserChoice。

windows/browser-guard.ps1：安装与卸载前按当前用户 SID 和精确引擎目录检查占用；完整脚本由 NSIS 释放到私有插件目录，通过短 -File 命令和原生 64 位 PowerShell 执行，路径经环境变量传递；真实不存在的引擎目录直接通过，查询未知或超时中止。
