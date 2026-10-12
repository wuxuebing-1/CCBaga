# Offline tests: isolated directories, fake package operations, no real installs.
$ErrorActionPreference = 'Stop'
. ([ScriptBlock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'claude-cli.ps1'))))
# Never query the machine's package managers from fixtures.
function Get-NpmInstalls { }
function Get-WinGet { return $null }
$nodeRuntime = (Get-Command node.exe -ErrorAction Stop).Source
function Assert($condition, [string]$message) { if (-not $condition) { throw $message } }
function Rejected([scriptblock]$operation) {
    $failed = $false
    try { & $operation } catch { $failed = $true }
    Assert $failed 'Expected refusal'
}
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('ccbaga-cli-test-' + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $fixture
$savedPath = $env:PATH
$savedRoot = $env:CC_CLI_USER_DIR
$savedAction = $env:CC_CLI_ACTION
try {
    $env:CC_CLI_USER_DIR = $fixture
    $env:PATH = $fixture
    $env:CC_CLI_ACTION = 'read'
    Main
    $launcher = Join-Path $fixture '.local\bin\claude.exe'
    $native = Join-Path $fixture '.local\share\claude'
    $null = New-Item -ItemType Directory -Path (Split-Path $launcher), (Join-Path $native 'versions'), (Join-Path $fixture '.claude\skills\custom'), (Join-Path $fixture 'projects')
    [IO.File]::WriteAllText($launcher, 'custom executable')
    Rejected { Get-Native $fixture }
    Assert (Test-Path -LiteralPath $launcher) 'Unknown launcher must survive'
    # Test fixture signs nothing: replace the binary verification function only
    # inside this test process, leaving production signature enforcement intact.
    function Assert-OfficialBinary([string]$Path) {
        Assert-PlainPath $Path
        Assert ([IO.File]::ReadAllText($Path) -eq 'fixture-official') 'Unexpected fixture binary'
    }
    [IO.File]::WriteAllText($launcher, 'fixture-official')
    [IO.File]::WriteAllText((Join-Path $native 'versions\1.0.0'), 'fixture-official')
    [IO.File]::WriteAllText((Join-Path $fixture '.claude\settings.json'), 'personal-settings')
    [IO.File]::WriteAllText((Join-Path $fixture '.claude\skills\custom\SKILL.md'), 'personal-skill')
    [IO.File]::WriteAllText((Join-Path $fixture 'projects\source.txt'), 'personal-source')
    [IO.File]::WriteAllText((Join-Path $fixture '.claude.json'), 'personal-state')
    [IO.File]::WriteAllText((Join-Path $native 'unexpected.txt'), 'keep-me')
    Rejected { Get-Native $fixture }
    Remove-Item -LiteralPath (Join-Path $native 'unexpected.txt')
    $env:CC_CLI_ACTION = 'uninstall'
    Main
    Assert (-not (Test-Path -LiteralPath $native)) 'Native program must be removed'
    Assert (-not (Test-Path -LiteralPath $launcher)) 'Native launcher must be removed'
    foreach ($asset in @('.claude\settings.json', '.claude\skills\custom\SKILL.md', 'projects\source.txt', '.claude.json')) {
        Assert (Test-Path -LiteralPath (Join-Path $fixture $asset)) ('Personal asset was deleted: ' + $asset)
    }
    # Reparse/junction traversal rejection does not need symlink privilege.
    $outside = Join-Path $fixture 'outside'
    $null = New-Item -ItemType Directory -Path $outside
    try {
        $null = New-Item -ItemType Junction -Path $native -Target $outside
        Rejected { Get-Native $fixture }
        [IO.Directory]::Delete($native)
    } catch [System.ComponentModel.Win32Exception] {
        if ($_.Exception.NativeErrorCode -notin @(5, 1314)) { throw }
        Write-Output 'Junction creation not exercised: Windows sandbox denied reparse creation.'
    }

    # Execute only npm's shim-generation library on disposable files; never npm.
    $prefix = Join-Path $fixture 'shim-fixture'
    $target = Join-Path $prefix 'node_modules\@anthropic-ai\claude-code\cli.js'
    $null = New-Item -ItemType Directory -Path (Split-Path $target)
    [IO.File]::WriteAllText($target, "#!/usr/bin/env node`nprocess.stdout.write('fixture');`n")
    $npmEntry = Join-Path (Split-Path $nodeRuntime) 'node_modules\npm\bin\npm-cli.js'
    $shimModule = Join-Path (Split-Path (Split-Path $npmEntry)) 'node_modules\cmd-shim'
    $null = Invoke-Tool $nodeRuntime @('-e', 'require(process.argv[1])(process.argv[2],process.argv[3]).catch(()=>process.exit(1))', $shimModule, $target, (Join-Path $prefix 'claude')) 'Fixture shim generation failed'
    $shimInstall = [pscustomobject]@{ Node=$nodeRuntime; Npm=$npmEntry; Prefix=$prefix; Path=$target; Relative='cli.js' }
    Assert-NpmShims $shimInstall
    [IO.File]::AppendAllText((Join-Path $prefix 'claude.cmd'), "`r`necho custom-wrapper`r`n")
    Rejected { Assert-NpmShims $shimInstall }
    Assert (Test-Path -LiteralPath (Join-Path $prefix 'claude.cmd')) 'Custom npm shim must survive'
    $nativeTarget = Join-Path (Split-Path $target) 'bin\claude.exe'
    $null = New-Item -ItemType Directory -Path (Split-Path $nativeTarget)
    [IO.File]::WriteAllBytes($nativeTarget, [byte[]]@(77, 90, 0, 0))
    $null = Invoke-Tool $nodeRuntime @('-e', 'require(process.argv[1])(process.argv[2],process.argv[3]).catch(()=>process.exit(1))', $shimModule, $nativeTarget, (Join-Path $prefix 'claude')) 'Native fixture shim generation failed'
    $shimInstall.Path = $nativeTarget
    $shimInstall.Relative = 'bin/claude.exe'
    Assert-NpmShims $shimInstall

    # Package manager fixture. Nothing in this branch invokes npm or WinGet.
    $script:installed = $true
    $script:operations = @()
    $script:fake = [pscustomobject]@{ Method='npm'; Node='fixture-node'; Npm='fixture-npm'; Prefix=$fixture; OriginalPrefix=$fixture; Package=(Join-Path $fixture 'fake-package'); Path=(Join-Path $fixture 'fake-cli.js'); Relative='cli.js' }
    function Get-NpmInstalls { if ($script:installed) { $script:fake } }
    function Get-WinGet { return $null }
    function Assert-NpmShims($Install) { }
    function Invoke-Tool([string]$Program, [string[]]$Arguments, [string]$Failure) {
        Assert ($Program -eq 'fixture-node') 'Only the fake package manager may run'
        if ($Arguments[0] -eq $script:fake.Path -and $Arguments[1] -eq '--version') { return '3.0.0 (Claude Code)' }
        Assert ($Arguments[0] -eq 'fixture-npm') 'Unexpected package manager'
        $script:operations += $Arguments[1]
        switch ($Arguments[1]) {
            'uninstall' { $script:installed = $false }
            'install' { $script:installed = $true }
            default { throw 'Unexpected package operation' }
        }
        return ''
    }
    $env:CC_CLI_ACTION = 'reinstall'
    Main
    Assert (($script:operations -join ',') -eq 'uninstall,install') 'Reinstall must preserve npm method'
    Assert $script:installed 'Fixture reinstall must complete'
    function Invoke-Tool { throw 'npm fixture failed' }
    Rejected { Main }
    $env:CC_CLI_ACTION = 'arbitrary'
    Rejected { Main }
    Write-Output 'Windows Claude CLI fixture tests passed.'
} finally {
    $env:PATH = $savedPath
    $env:CC_CLI_USER_DIR = $savedRoot
    $env:CC_CLI_ACTION = $savedAction
    # Exact GUID directory created above; do not follow a fixture junction.
    $native = Join-Path $fixture '.local\share\claude'
    if ((Test-Path -LiteralPath $native) -and ((Get-Item -LiteralPath $native).Attributes -band [IO.FileAttributes]::ReparsePoint)) { [IO.Directory]::Delete($native) }
    if ([IO.Path]::GetFullPath($fixture).StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath()), [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
