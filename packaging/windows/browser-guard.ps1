$ErrorActionPreference='Stop'
try {
 $value=$env:CCBAGA_INSTALL_ENGINE
 if ([string]::IsNullOrWhiteSpace($value) -or -not [IO.Path]::IsPathRooted($value)) { exit 13 }
 $root=[IO.Path]::GetFullPath($value).TrimEnd('\')+'\'
 # A genuinely absent engine directory is a fresh install; unrelated browsers
 # cannot have loaded files from it. Permission errors are not treated as absence.
 try { $attributes=[IO.File]::GetAttributes($value) }
 catch [IO.FileNotFoundException] { exit 0 }
 catch [IO.DirectoryNotFoundException] { exit 0 }
 if (($attributes -band [IO.FileAttributes]::Directory) -eq 0) { exit 13 }
 $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
 foreach($process in @(Get-CimInstance Win32_Process -ErrorAction Stop)) {
  if (-not $process.ExecutablePath) {
   # An unreadable Chromium image for this user cannot prove the engine is idle.
   if ($process.Name -in @('chrome.exe','chromium.exe','CCBaga-B.exe')) {
    $owner=Invoke-CimMethod -InputObject $process -MethodName GetOwnerSid -ErrorAction Stop
    if ($owner.ReturnValue -ne 0 -or $owner.Sid -eq $sid) { exit 13 }
   }
   continue
  }
  $path=[IO.Path]::GetFullPath([string]$process.ExecutablePath)
  if (-not $path.StartsWith($root,[StringComparison]::OrdinalIgnoreCase)) { continue }
  $owner=Invoke-CimMethod -InputObject $process -MethodName GetOwnerSid -ErrorAction Stop
  if ($owner.ReturnValue -ne 0 -or -not $owner.Sid) { exit 13 }
  if ($owner.Sid -eq $sid) { exit 12 }
 }
 exit 0
} catch { exit 13 }
