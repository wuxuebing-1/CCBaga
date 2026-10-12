# [INPUT]: Embedded host action and current user's Windows environment.
# [OUTPUT]: Real CLI stage/status markers; native, npm and WinGet program-only maintenance.
# [POS]: Preserves settings, projects, skills and credentials; refuses unknown launchers/reparse trees.
# [PROTOCOL]: Update this header and check CLAUDE.md when changing the boundary.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Fail([string]$Message) { throw $Message }
function Step([string]$Message) { [Console]::Out.WriteLine('CC_CLI_STEP=' + $Message) }

function Assert-PlainPath([string]$Path) {
    if (-not [IO.Path]::IsPathRooted($Path)) { Fail 'Claude Code 安装路径无效。' }
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -Force -LiteralPath $current
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                Fail 'Claude Code 安装目录包含外部链接，已停止操作。'
            }
        }
        $parent = [IO.Directory]::GetParent($current)
        $current = if ($parent) { $parent.FullName } else { $null }
    }
}

function Assert-OfficialBinary([string]$Path) {
    Assert-PlainPath $Path
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { Fail 'Claude Code 程序文件缺失。' }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate -or
        $signature.SignerCertificate.GetNameInfo([Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false) -ne 'Anthropic, PBC') {
        Fail 'Claude Code 启动器是自定义文件或签名无法核验，已停止操作。'
    }
}

function Invoke-Tool([string]$Program, [string[]]$Arguments, [string]$Failure) {
    $script:LASTEXITCODE = 0
    $savedPreference = $ErrorActionPreference
    try {
        # Native stderr may contain credentials; never return it to the webview.
        $ErrorActionPreference = 'Continue'
        $output = @(& $Program @Arguments 2>$null)
        $exitCode = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    if ($exitCode -ne 0) { Fail $Failure }
    return ($output -join "`n").Trim()
}

function Get-Native([string]$Root) {
    $directory = Join-Path $Root '.local\share\claude'
    $launcher = Join-Path $Root '.local\bin\claude.exe'
    Assert-PlainPath $directory
    Assert-PlainPath $launcher
    $exists = (Test-Path -LiteralPath $directory) -or (Test-Path -LiteralPath $launcher)
    if (-not $exists) { return $null }
    if (Test-Path -LiteralPath $launcher) { Assert-OfficialBinary $launcher }
    if (Test-Path -LiteralPath $directory) {
        # The documented native layout contains signed version files only.
        foreach ($item in @(Get-ChildItem -Force -LiteralPath $directory)) {
            if ($item.Name -ne 'versions' -or -not $item.PSIsContainer) { Fail 'Claude Code 原生目录含未识别文件，已停止操作。' }
            Assert-PlainPath $item.FullName
            foreach ($version in @(Get-ChildItem -Force -LiteralPath $item.FullName)) {
                if ($version.PSIsContainer) { Fail 'Claude Code 原生版本目录含未识别文件，已停止操作。' }
                Assert-OfficialBinary $version.FullName
            }
        }
    }
    return [pscustomobject]@{ Method = 'native'; Path = $launcher; Directory = $directory }
}

function Assert-NpmShims($Install) {
    # Recreate npm's own generated shims in a disposable directory and compare
    # all three, so uninstall never silently removes a custom wrapper.
    $checker = @'
const fs=require('fs'),os=require('os'),path=require('path');
const [source,prefix,shimModule,relative]=process.argv.slice(1);
const temp=fs.realpathSync(os.tmpdir()),base=fs.mkdtempSync(path.join(temp,'ccbaga-npm-check-'));
(async()=>{try {
 const bin=path.join(base,'node_modules','@anthropic-ai','claude-code',relative);
 fs.mkdirSync(path.dirname(bin),{recursive:true});
 const fd=fs.openSync(source,'r'); const bytes=Buffer.alloc(512); const size=fs.readSync(fd,bytes,0,bytes.length,0); fs.closeSync(fd);
 fs.writeFileSync(bin,bytes.subarray(0,size));
 await require(shimModule)(bin,path.join(base,'claude'));
 for(const suffix of ['', '.cmd', '.ps1']) {
  const actual=path.join(prefix,'claude'+suffix);
  if(!fs.existsSync(actual))continue;
  if(fs.lstatSync(actual).isSymbolicLink())throw Error('link');
  const normalize=s=>s.replace(/\r\n/g,'\n').trimEnd();
  if(normalize(fs.readFileSync(actual,'utf8'))!==normalize(fs.readFileSync(path.join(base,'claude'+suffix),'utf8')))throw Error('custom');
 }
}finally{if(path.dirname(base)!==temp)throw Error('unsafe cleanup');fs.rmSync(base,{recursive:true,force:true})}})().catch(()=>process.exit(1));
'@
    foreach ($suffix in @('', '.cmd', '.ps1', '.exe')) { Assert-PlainPath (Join-Path $Install.Prefix ('claude' + $suffix)) }
    if (Test-Path -LiteralPath (Join-Path $Install.Prefix 'claude.exe')) { Fail 'npm 启动目录存在未识别的 claude.exe，已停止操作。' }
    $shim = Join-Path (Split-Path (Split-Path $Install.Npm)) 'node_modules\cmd-shim'
    if (-not (Test-Path -LiteralPath $shim)) { Fail 'npm 缺少启动器核验组件，已停止操作。' }
    $null = Invoke-Tool $Install.Node @('-e', $checker, $Install.Path, $Install.Prefix, $shim, $Install.Relative) 'npm 启动器是自定义文件或外部链接，已停止操作。'
}

function Get-NpmInstalls {
    $nodes = @((Get-Command node.exe -All -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source))
    if ($env:NVM_HOME -and (Test-Path -LiteralPath $env:NVM_HOME)) {
        $nodes += @(Get-ChildItem -LiteralPath $env:NVM_HOME -Directory -Filter 'v*' | ForEach-Object { Join-Path $_.FullName 'node.exe' })
    }
    $seen = @{}
    foreach ($node in @($nodes | Select-Object -Unique)) {
        if (-not (Test-Path -LiteralPath $node -PathType Leaf)) { continue }
        # Resolve the NVM symlink for Node; installation targets themselves must be plain paths.
        $npm = Join-Path (Split-Path $node) 'node_modules\npm\bin\npm-cli.js'
        if (-not (Test-Path -LiteralPath $npm -PathType Leaf)) { continue }
        $prefix = Invoke-Tool $node @($npm, 'prefix', '-g') 'npm 无法读取全局安装目录。'
        if (-not [IO.Path]::IsPathRooted($prefix) -or $prefix.Contains("`n")) { Fail 'npm 返回了无效的全局目录。' }
        $originalPrefix = [IO.Path]::GetFullPath($prefix)
        if (-not (Test-Path -LiteralPath (Join-Path $originalPrefix 'node_modules\@anthropic-ai\claude-code\package.json'))) { continue }
        $prefix = Invoke-Tool $node @('-e', "console.log(require('fs').realpathSync(process.argv[1]))", $originalPrefix) 'npm 无法核验全局目录。'
        if ($seen.ContainsKey($prefix)) { continue }
        $seen[$prefix] = $true
        $package = Join-Path $prefix 'node_modules\@anthropic-ai\claude-code'
        $manifest = Join-Path $package 'package.json'
        if (-not (Test-Path -LiteralPath $manifest)) { continue }
        Assert-PlainPath $manifest
        $metadata = Get-Content -Raw -LiteralPath $manifest | ConvertFrom-Json
        if ($metadata.name -ne '@anthropic-ai/claude-code') { Fail 'npm Claude Code 包标识不匹配。' }
        $relative = if ($metadata.bin -is [string]) { $metadata.bin } else { $metadata.bin.claude }
        if (-not $relative -or $relative -match '(^[\\/]|:|(^|[\\/])\.\.([\\/]|$))') { Fail 'npm Claude Code 程序入口无效。' }
        $binary = [IO.Path]::GetFullPath((Join-Path $package $relative))
        Assert-PlainPath $binary
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { Fail 'npm Claude Code 程序入口缺失。' }
        if ([IO.Path]::GetExtension($binary) -eq '.exe') { Assert-OfficialBinary $binary }
        elseif ([IO.Path]::GetExtension($binary) -ne '.js') { Fail 'npm Claude Code 程序入口无法识别。' }
        $install = [pscustomobject]@{ Method = 'npm'; Node = $node; Npm = $npm; Prefix = $prefix; OriginalPrefix = $originalPrefix; Package = $package; Path = $binary; Relative = $relative }
        Assert-NpmShims $install
        $install
    }
}

function Get-WinGet {
    $manager = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $manager) { return $null }
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $text = (& $manager.Source list --id Anthropic.ClaudeCode --exact --disable-interactivity 2>$null | Out-String)
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    if ($code -eq -1978335212) { return $null } # APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND
    if ($code -ne 0) { Fail 'WinGet 无法读取安装状态，请先在终端检查 WinGet。' }
    if ($text -notmatch '(?m)\bAnthropic\.ClaudeCode\s') { Fail 'WinGet 返回了无法识别的安装状态。' }
    $bases = @((Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet'))
    if ($env:ProgramFiles) { $bases += Join-Path $env:ProgramFiles 'WinGet' }
    $paths = @(); $links = @()
    foreach ($base in $bases) {
        $link = Join-Path $base 'Links\claude.exe'
        if (Test-Path -LiteralPath $link) {
            $item = Get-Item -Force -LiteralPath $link
            $path = if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                [IO.Path]::GetFullPath(@($item.Target)[0])
            } else { $link }
            $paths += $path
            $links += $link
        } else {
            $paths += @(Get-ChildItem -LiteralPath (Join-Path $base 'Packages') -Directory -Filter 'Anthropic.ClaudeCode_*' -ErrorAction SilentlyContinue |
                ForEach-Object { Join-Path $_.FullName 'claude.exe' } | Where-Object { Test-Path -LiteralPath $_ })
        }
    }
    $paths = @($paths | Select-Object -Unique)
    if (-not $paths.Count) { Fail 'WinGet Claude Code 启动器位置无法核验。' }
    foreach ($path in $paths) {
        $allowed = @($bases | Where-Object { $path.StartsWith(([IO.Path]::GetFullPath((Join-Path $_ 'Packages')) + '\Anthropic.ClaudeCode_'), [StringComparison]::OrdinalIgnoreCase) })
        if (-not $allowed.Count) { Fail 'WinGet Claude Code 启动器位置无法核验。' }
        Assert-OfficialBinary $path
    }
    return [pscustomobject]@{ Method = 'WinGet'; Manager = $manager.Source; Path = $paths[0]; Paths = $paths; Links = $links }
}

function Assert-KnownLaunchers($Installs) {
    $allowed = @()
    foreach ($install in $Installs) {
        $allowed += $install.Path
        if ($install.Method -eq 'npm') {
            foreach ($suffix in @('', '.cmd', '.ps1')) {
                $allowed += Join-Path $install.Prefix ('claude' + $suffix)
                $allowed += Join-Path $install.OriginalPrefix ('claude' + $suffix)
            }
        } elseif ($install.Method -eq 'WinGet') { $allowed += $install.Links; $allowed += $install.Paths }
    }
    foreach ($command in @(Get-Command claude,claude.exe,claude.cmd,claude.ps1 -All -ErrorAction SilentlyContinue)) {
        if ($command.CommandType -notin @('Application', 'ExternalScript') -or $command.Source -notin $allowed) {
            Fail '发现未识别的 Claude Code 启动器，已停止操作。'
        }
    }
}

function Get-Installs([string]$Root) {
    $all = @()
    $native = Get-Native $Root
    if ($native) { $all += $native }
    $all += @(Get-NpmInstalls)
    $winget = Get-WinGet
    if ($winget) { $all += $winget }
    Assert-KnownLaunchers $all
    return $all
}

function Get-Version($Install) {
    $env:DISABLE_AUTOUPDATER = '1'
    $version = if ($Install.Method -eq 'npm' -and [IO.Path]::GetExtension($Install.Path) -eq '.js') {
        Invoke-Tool $Install.Node @($Install.Path, '--version') 'Claude Code 版本核验失败。'
    } else { Invoke-Tool $Install.Path @('--version') 'Claude Code 版本核验失败。' }
    if ($version -notmatch '^\d+\.\d+\.\d+[^\r\n]*\(Claude Code\)$') { Fail 'Claude Code 版本信息无效。' }
    return $version
}

function Report($Installs) {
    $version = ''; $path = ''
    if ($Installs.Count) { $path = $Installs[0].Path; $version = Get-Version $Installs[0] }
    [Console]::Out.WriteLine('CC_CLI_METHOD=' + ((@($Installs | ForEach-Object { $_.Method } | Select-Object -Unique)) -join ' / '))
    [Console]::Out.WriteLine('CC_CLI_PATH=' + $path)
    [Console]::Out.WriteLine('CC_CLI_VERSION=' + $version)
    [Console]::Out.WriteLine('CC_CLI_DONE=1')
}

function Main {
    $action = $env:CC_CLI_ACTION
    if ($action -notin @('read', 'uninstall', 'reinstall')) { Fail 'Claude Code 无效的安装操作。' }
    $root = [IO.Path]::GetFullPath($env:CC_CLI_USER_DIR)
    Assert-PlainPath $root
    $installs = @(Get-Installs $root)
    if ($action -eq 'read') { Report $installs; return }
    $preferred = @($installs | Where-Object Method -eq 'npm' | Select-Object -First 1)
    if (-not $preferred.Count) { $preferred = @($installs | Where-Object Method -eq 'WinGet' | Select-Object -First 1) }
    $installer = ''
    if ($action -eq 'reinstall' -and -not $preferred.Count) {
        Step '正在下载 Claude Code 官方安装器。'
        $curl = Join-Path $env:SystemRoot 'System32\curl.exe'
        $installer = Invoke-Tool $curl @('--fail', '--silent', '--show-error', '--location', '--proto', '=https', '--proto-redir', '=https', '--max-time', '60', 'https://claude.ai/install.ps1') 'Claude Code 官方安装器下载失败。'
        if ($installer -isnot [string] -or $installer.Length -gt 65536 -or $installer -notmatch '^\s*param\(') { Fail 'Claude Code 官方安装器内容无效。' }
        # Parse before uninstall so a bad response cannot remove a working CLI.
        $null = [ScriptBlock]::Create($installer)
    }
    Step '正在卸载 Claude Code CLI。'
    foreach ($install in $installs) {
        switch ($install.Method) {
            'npm' {
                Assert-NpmShims $install
                $null = Invoke-Tool $install.Node @($install.Npm, 'uninstall', '-g', '@anthropic-ai/claude-code') 'npm 卸载失败，操作可能已部分完成。'
                if (Test-Path -LiteralPath $install.Package) { Fail 'npm 卸载后仍存在 Claude Code 程序。' }
            }
            'WinGet' {
                $null = Invoke-Tool $install.Manager @('uninstall', '--id', 'Anthropic.ClaudeCode', '--exact', '--disable-interactivity') 'WinGet 卸载失败，操作可能已部分完成。'
            }
            'native' {
                $checked = Get-Native $root
                if (-not $checked -or $checked.Directory -ne $install.Directory) { Fail 'Claude Code 原生安装已变化。' }
                if (Test-Path -LiteralPath $checked.Path) { Remove-Item -LiteralPath $checked.Path -Force }
                if (Test-Path -LiteralPath $checked.Directory) { Remove-Item -LiteralPath $checked.Directory -Recurse -Force }
            }
        }
    }
    if (@(Get-Installs $root).Count) { Fail '仍有其他 Claude Code 安装，未完成卸载。' }
    if ($action -eq 'uninstall') { Report @(); return }
    Step '正在重装 Claude Code CLI。'
    if ($preferred.Count -and $preferred[0].Method -eq 'npm') {
        $install = $preferred[0]
        $null = Invoke-Tool $install.Node @($install.Npm, 'install', '-g', '@anthropic-ai/claude-code@latest') 'npm 重装失败，操作可能已部分完成。'
    } elseif ($preferred.Count) {
        $null = Invoke-Tool $preferred[0].Manager @('install', '--id', 'Anthropic.ClaudeCode', '--exact', '--disable-interactivity', '--accept-package-agreements', '--accept-source-agreements') 'WinGet 重装失败，操作可能已部分完成。'
    } else {
        # The official installer validates its download before executing it.
        # It can call exit: any premature exit lacks our required DONE receipt.
        & ([ScriptBlock]::Create($installer)) latest *> $null
        if ($LASTEXITCODE -ne 0) { Fail 'Claude Code 原生重装失败，操作可能已部分完成。' }
    }
    Step '正在核验 Claude Code CLI 版本。'
    $installed = @(Get-Installs $root)
    if (-not $installed.Count) { Fail 'Claude Code 重装后未找到可执行程序。' }
    Report $installed
}

# Dot-source in offline fixtures to exercise functions without executing Main.
if ($MyInvocation.InvocationName -ne '.') {
    try { Main }
    catch {
        $message = $_.Exception.Message
        if ($message -notmatch '^(Claude Code |npm |WinGet |发现未识别的 |仍有其他 )') { $message = 'Claude Code 安装任务失败，操作可能已部分完成，请检查安装目录权限。' }
        [Console]::Error.WriteLine($message)
        exit 1
    }
}
