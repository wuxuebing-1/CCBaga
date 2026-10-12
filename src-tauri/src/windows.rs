// [INPUT]: Windows current-user tokens, process/file handles, credential metadata and system preferences.
// [OUTPUT]: Verified Windows system operations and the same JSON bridge used by macOS/Linux.
// [POS]: Native boundary: no credential blob dereferences, command lines never reach the WebView,
//        location/default app choices remain in Windows Settings; reads never change preferences.
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString, OsStr},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, Credentials::*, *},
    Storage::FileSystem::*,
    System::{DataExchange::*, Diagnostics::ToolHelp::*, Memory::*, Threading::*},
    UI::{Input::KeyboardAndMouse::GetActiveWindow, Shell::*, WindowsAndMessaging::*},
};

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
unsafe fn from_wide(value: *const u16) -> String {
    if value.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *value.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(value, len))
}
fn win_error(action: &str) -> String {
    format!("Windows: {action}: {}", std::io::Error::last_os_error())
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn handle(raw: HANDLE, action: &str) -> Result<Handle, String> {
    if raw.is_null() || raw == INVALID_HANDLE_VALUE {
        Err(win_error(action))
    } else {
        Ok(Handle(raw))
    }
}
fn response(value: Result<Value, String>) -> *mut c_char {
    CString::new(
        value
            .unwrap_or_else(|error| json!({"error":error}))
            .to_string(),
    )
    .expect("JSON escapes NUL")
    .into_raw()
}
unsafe fn input<'a>(value: *const c_char) -> &'a str {
    CStr::from_ptr(value).to_str().unwrap_or("")
}
pub unsafe fn rf_free(value: *mut c_char) {
    drop(CString::from_raw(value));
}

/// Values enter the child environment, never PowerShell source. Callers provide fixed scripts.
pub fn powershell(script: &str, vars: &[(&str, &str)]) -> Result<String, String> {
    let system = std::env::var_os("SystemRoot").ok_or("Windows directory is unavailable.")?;
    let output = Command::new(Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
        .arg(format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); try {{ {script} }} catch {{ [Console]::Error.WriteLine(('CCBAGA_SYSTEM_ERROR '+$_.CategoryInfo.Category+' HRESULT '+$_.Exception.HResult)); exit 1 }}"))
        .envs(vars.iter().copied())
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null()).output().map_err(|e| format!("Windows PowerShell: {e}"))?;
    if !output.status.success() {
        // Script diagnostics must not contain process arguments or credential contents.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr
            .lines()
            .find_map(|line| line.strip_prefix("CCBAGA_SYSTEM_ERROR "))
            .unwrap_or("no system error category returned");
        return Err(format!("Windows system operation failed ({}; {reason}). Check Windows permissions and policy; preferences may have changed, so refresh the current state before retrying.", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .into())
}

pub fn open_url(url: &str) -> Result<(), String> {
    if url.contains('\0') || !(url.starts_with("https://") || url.starts_with("ms-settings:")) {
        return Err("Invalid Windows settings or HTTPS address.".into());
    }
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide(url).as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        Err(format!(
            "Windows could not open the requested page ({}).",
            result as isize
        ))
    } else {
        Ok(())
    }
}
pub fn open_settings(pane: &str) -> Result<Value, String> {
    let uri = match pane {
        "timezone" => "ms-settings:dateandtime",
        "language" => "ms-settings:regionlanguage",
        "location" => "ms-settings:privacy-location",
        "default" => "ms-settings:defaultapps",
        _ => return Err("Invalid settings panel.".into()),
    };
    open_url(uri)?;
    Ok(json!({"ok":true}))
}

pub unsafe fn rf_copy(text: *const c_char) -> Result<(), String> {
    let text = wide(input(text));
    let memory = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2);
    if memory.is_null() {
        return Err(win_error("allocate clipboard"));
    }
    let ptr = GlobalLock(memory);
    if ptr.is_null() {
        GlobalFree(memory);
        return Err(win_error("lock clipboard"));
    }
    std::ptr::copy_nonoverlapping(text.as_ptr(), ptr.cast::<u16>(), text.len());
    GlobalUnlock(memory);
    let owner = GetActiveWindow();
    if owner.is_null() || OpenClipboard(owner) == 0 {
        GlobalFree(memory);
        return Err(win_error(
            "open clipboard (requires an active application window)",
        ));
    }
    let result = if EmptyClipboard() == 0 || SetClipboardData(13, memory).is_null() {
        GlobalFree(memory);
        Err(win_error("copy text"))
    } else {
        Ok(())
    };
    CloseClipboard();
    result
}

// CredEnumerateW returns OS-owned credential records. Only target/type/LastWritten are
// inspected; CredentialBlob, user name, attributes and passwords are never dereferenced.
fn allowed_credential(target: &str) -> bool {
    target == "Claude Code-credentials"
        || target.starts_with("Claude Code-credentials-")
        || target == "Claude Safe Storage"
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialReference {
    target: String,
    kind: u32,
    written: u64,
}
fn filetime(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}
fn credential_metadata() -> Result<Vec<CredentialReference>, String> {
    let mut items = Vec::new();
    // Filter before enumeration, so unrelated credential records are not requested.
    for filter in ["Claude Code-credentials*", "Claude Safe Storage"] {
        let mut count = 0;
        let mut records = null_mut();
        unsafe {
            if CredEnumerateW(wide(filter).as_ptr(), 0, &mut count, &mut records) == 0 {
                if GetLastError() == ERROR_NOT_FOUND {
                    continue;
                }
                return Err(win_error("enumerate Credential Manager metadata"));
            }
            for ptr in std::slice::from_raw_parts(records, count as usize) {
                let record = &**ptr;
                let target = from_wide(record.TargetName);
                if record.Type == CRED_TYPE_GENERIC && allowed_credential(&target) {
                    items.push(CredentialReference {
                        target,
                        kind: record.Type,
                        written: filetime(record.LastWritten),
                    });
                }
            }
            CredFree(records.cast());
        }
    }
    Ok(items)
}
pub unsafe fn rf_credentials_list() -> *mut c_char {
    response(credential_metadata().map(|items| json!({"items":items.into_iter().map(|item| {
        json!({"service":item.target,"reference":serde_json::to_string(&item).expect("serializable metadata")})
    }).collect::<Vec<_>>()})))
}
pub unsafe fn rf_credentials_remove(reference: *const c_char) -> *mut c_char {
    response((|| {
        let requested: CredentialReference = serde_json::from_str(input(reference))
            .map_err(|_| "Invalid credential reference; scan again.")?;
        if requested.kind != CRED_TYPE_GENERIC
            || !allowed_credential(&requested.target)
            || !credential_metadata()?.contains(&requested)
        {
            return Err(
                "Credential identity changed or is outside the Claude allowlist; scan again."
                    .into(),
            );
        }
        if CredDeleteW(wide(&requested.target).as_ptr(), requested.kind, 0) == 0 {
            return Err(win_error("delete selected credential"));
        }
        Ok(json!({"ok":true}))
    })())
}
pub unsafe fn rf_keychain_unlock() -> *mut c_char {
    response(Ok(
        json!({"message":"Windows Credential Manager uses the signed-in user's session. There is no separate keychain to unlock; credential cleanup still requires a fresh confirmed scan."}),
    ))
}

fn token_user(process: HANDLE) -> Result<Vec<u64>, String> {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return Err(win_error("read process owner"));
        }
        let token = Handle(token);
        let mut size = 0;
        GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut size);
        if size == 0 {
            return Err(win_error("read token size"));
        }
        let mut bytes = vec![0u64; (size as usize).div_ceil(8)];
        if GetTokenInformation(
            token.0,
            TokenUser,
            bytes.as_mut_ptr().cast(),
            size,
            &mut size,
        ) == 0
        {
            return Err(win_error("read token identity"));
        }
        Ok(bytes)
    }
}
unsafe fn sid(bytes: &[u64]) -> PSID {
    (*(bytes.as_ptr().cast::<TOKEN_USER>())).User.Sid
}
fn current_user_process(process: HANDLE) -> bool {
    match (
        token_user(process),
        token_user(unsafe { GetCurrentProcess() }),
    ) {
        (Ok(target), Ok(current)) => unsafe { EqualSid(sid(&target), sid(&current)) != 0 },
        _ => false,
    }
}

pub fn owned_by_current_user(path: &Path) -> bool {
    unsafe {
        let Ok(file) = handle(
            CreateFileW(
                wide(path).as_ptr(),
                READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            ),
            "open file owner",
        ) else {
            return false;
        };
        let Ok(current) = token_user(GetCurrentProcess()) else {
            return false;
        };
        let mut owner = null_mut();
        let mut descriptor = null_mut();
        let status = GetSecurityInfo(
            file.0,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        );
        if status != 0 {
            return false;
        }
        let matches = !owner.is_null() && EqualSid(owner, sid(&current)) != 0;
        LocalFree(descriptor.cast());
        matches
    }
}
pub fn file_identity(path: &Path) -> Result<(u64, u64), String> {
    unsafe {
        let file = handle(
            CreateFileW(
                wide(path).as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            ),
            "open file identity",
        )?;
        let mut info = std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>();
        if GetFileInformationByHandle(file.0, &mut info) == 0 {
            return Err(win_error("read file identity"));
        }
        Ok((
            info.dwVolumeSerialNumber as u64,
            ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        ))
    }
}
pub fn file_change_time(path: &Path) -> Result<u64, String> {
    unsafe {
        let file = handle(
            CreateFileW(
                wide(path).as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            ),
            "open file change time",
        )?;
        let mut info = std::mem::zeroed::<FILE_BASIC_INFO>();
        if GetFileInformationByHandleEx(
            file.0,
            FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        ) == 0
        {
            return Err(win_error("read file change time"));
        }
        Ok(info.ChangeTime as u64)
    }
}
fn process_identity(process: HANDLE) -> Result<(PathBuf, String), String> {
    unsafe {
        let mut path = vec![0u16; 32768];
        let mut len = path.len() as u32;
        if QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut len) == 0 {
            return Err(win_error("read process executable"));
        }
        let (mut created, mut exited, mut kernel, mut user) = (
            std::mem::zeroed(),
            std::mem::zeroed(),
            std::mem::zeroed(),
            std::mem::zeroed(),
        );
        if GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) == 0 {
            return Err(win_error("read process identity"));
        }
        Ok((
            PathBuf::from(String::from_utf16_lossy(&path[..len as usize])),
            filetime(created).to_string(),
        ))
    }
}
#[derive(Debug)]
pub struct Process {
    pub pid: i32,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub started: String,
}
pub fn processes() -> Result<Vec<Process>, String> {
    let mut processes = Vec::new();
    unsafe {
        let snapshot = handle(
            CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0),
            "enumerate processes",
        )?;
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = Process32FirstW(snapshot.0, &mut entry);
        while found != 0 {
            if entry.th32ProcessID > 4 && entry.th32ProcessID <= i32::MAX as u32 {
                if let Ok(process) = handle(
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID),
                    "open process",
                ) {
                    if current_user_process(process.0) {
                        if let Ok((executable, started)) = process_identity(process.0) {
                            processes.push(Process {
                                pid: entry.th32ProcessID as i32,
                                executable,
                                args: Vec::new(),
                                started,
                            });
                        }
                    }
                }
            }
            found = Process32NextW(snapshot.0, &mut entry);
        }
    }
    if processes.is_empty() {
        return Ok(processes);
    }
    let filter = processes
        .iter()
        .map(|p| format!("ProcessId = {}", p.pid))
        .collect::<Vec<_>>()
        .join(" OR ");
    let raw = powershell("$rows=@(Get-CimInstance Win32_Process -Filter $env:CCBAGA_PROCESS_FILTER | Select-Object ProcessId,CommandLine); ConvertTo-Json -InputObject $rows -Compress", &[("CCBAGA_PROCESS_FILTER", &filter)])?;
    let rows: Vec<Value> =
        serde_json::from_str(&raw).map_err(|_| "Invalid Windows process metadata.")?;
    for process in &mut processes {
        if let Some(line) = rows
            .iter()
            .find(|r| r["ProcessId"].as_i64() == Some(process.pid as i64))
            .and_then(|r| r["CommandLine"].as_str())
        {
            process.args = parse_command_line(line);
        }
    }
    // A PID may have been reused while CIM was queried: fail closed on stale identities.
    processes.retain(|p| verified_process(p, PROCESS_QUERY_LIMITED_INFORMATION).is_ok());
    Ok(processes)
}
fn parse_command_line(line: &str) -> Vec<String> {
    if line.is_empty() {
        return Vec::new();
    }
    unsafe {
        let mut count = 0;
        let args = CommandLineToArgvW(wide(line).as_ptr(), &mut count);
        if args.is_null() {
            return Vec::new();
        }
        let result = std::slice::from_raw_parts(args, count as usize)
            .iter()
            .map(|p| from_wide(*p))
            .collect();
        LocalFree(args.cast());
        result
    }
}
fn same_path(a: &Path, b: &Path) -> bool {
    path_key(a) == path_key(b)
}
fn same_profile_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => same_path(&a, &b),
        // Before the first browser launch the profile may not exist yet.
        _ => same_path(a, b),
    }
}
fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches(r"\\?\")
        .trim_end_matches('\\')
        .to_lowercase()
}
fn relative_parts(path: &Path, root: &Path) -> Option<Vec<String>> {
    let key = path_key(path);
    let root = format!("{}\\", path_key(root));
    let parts: Vec<String> = key
        .strip_prefix(&root)?
        .split('\\')
        .map(str::to_owned)
        .collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    Some(parts)
}
fn version_name(value: &str) -> bool {
    let value = value.strip_suffix(".exe").unwrap_or(value);
    value.split('.').count() == 3
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()))
}
/// Current-user package registration binds the exact Claude publisher family to its
/// actual installation directory, including an installation moved to another drive.
pub fn claude_desktop_packages() -> Result<Vec<PathBuf>, String> {
    let raw = powershell("$paths=@(Get-AppxPackage -Name Claude | Where-Object { $_.PackageFamilyName -ceq 'Claude_pzs8sxrjxfjjc' } | ForEach-Object { [string]$_.InstallLocation }); ConvertTo-Json -InputObject $paths -Compress", &[])?;
    let paths: Vec<String> = serde_json::from_str(&raw)
        .map_err(|_| "Cannot verify Claude's Windows package registration.")?;
    Ok(paths
        .into_iter()
        .filter(|p| !p.is_empty() && Path::new(p).is_absolute())
        .map(PathBuf::from)
        .collect())
}
pub fn claude_kind(
    process: &Process,
    home: &Path,
    local: &Path,
    packages: &[PathBuf],
) -> Option<&'static str> {
    if node_cli(process) {
        return Some("Claude Code");
    }
    if same_path(&process.executable, &home.join(".local/bin/claude.exe")) {
        return Some("Claude Code");
    }
    if relative_parts(
        &process.executable,
        &home.join(".local/share/claude/versions"),
    )
    .is_some_and(|parts| parts.len() == 1 && version_name(&parts[0]))
    {
        return Some("Claude Code");
    }
    if relative_parts(
        &process.executable,
        &local.join("Microsoft/WinGet/Packages"),
    )
    .is_some_and(|parts| {
        parts.len() == 2
            && parts[0] == "anthropic.claudecode_microsoft.winget.source_8wekyb3d8bbwe"
            && parts[1] == "claude.exe"
    }) {
        return Some("Claude Code");
    }
    if packages.iter().any(|root| {
        relative_parts(&process.executable, root)
            .is_some_and(|parts| parts.last().is_some_and(|name| name.ends_with(".exe")))
    }) {
        return Some("Claude Desktop");
    }
    // Squirrel's historical user install. All application helpers inside a numeric
    // app-version directory belong to the same dedicated installation; data folders,
    // arbitrary similarly named launchers and neighboring directories do not.
    if relative_parts(&process.executable, &local.join("AnthropicClaude")).is_some_and(|parts| {
        (parts.len() == 1 && parts[0] == "claude.exe")
            || (parts.len() >= 2
                && parts[0].strip_prefix("app-").is_some_and(version_name)
                && parts.last().is_some_and(|name| name.ends_with(".exe")))
    }) {
        return Some("Claude Desktop");
    }
    None
}
fn verified_process(target: &Process, access: u32) -> Result<Handle, String> {
    let process = unsafe {
        handle(
            OpenProcess(access, 0, target.pid as u32),
            "open selected process",
        )?
    };
    let (executable, started) = process_identity(process.0)?;
    if !current_user_process(process.0)
        || !same_path(&executable, &target.executable)
        || started != target.started
    {
        return Err("Process identity changed; scan again.".into());
    }
    Ok(process)
}
pub fn terminate_process(target: &Process) -> Result<(), String> {
    let process = verified_process(
        target,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
    )?;
    if unsafe { TerminateProcess(process.0, 1) } == 0 {
        Err(win_error("terminate selected process"))
    } else {
        Ok(())
    }
}
pub fn node_cli(process: &Process) -> bool {
    let name = process
        .executable
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if !["node.exe", "bun.exe"].contains(&name.as_str()) {
        return false;
    }
    for (index, arg) in process.args.iter().enumerate().skip(1) {
        if arg.starts_with("-e")
            || arg.starts_with("-p")
            || arg.starts_with("--eval")
            || arg.starts_with("--print")
        {
            return false;
        }
        if arg.starts_with('-') {
            if [
                "--",
                "--no-warnings",
                "--enable-source-maps",
                "--trace-warnings",
            ]
            .contains(&arg.as_str())
                || arg.starts_with("--max-old-space-size=")
                || arg.starts_with("--stack-size=")
            {
                continue;
            }
            // --require/--import/--loader and unknown options may consume the next
            // path as an option value rather than execute it as the CLI entry.
            return false;
        }
        if name == "bun.exe" && index == 1 && arg == "run" {
            continue;
        }
        // Relative entry points cannot be tied safely to a process working directory.
        return Path::new(arg).is_absolute()
            && arg
                .replace('\\', "/")
                .to_lowercase()
                .ends_with("/node_modules/@anthropic-ai/claude-code/cli.js");
    }
    false
}
fn profile_processes(profile: &str) -> Result<Vec<Process>, String> {
    let all = processes()?;
    if all.iter().any(|p| {
        let name = p
            .executable
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        ["chrome.exe", "chromium.exe", "ccbaga-b.exe"].contains(&name.as_str()) && p.args.is_empty()
    }) {
        return Err("Cannot verify a running Chromium process's profile. Close its windows and scan again before modifying browser data.".into());
    }
    Ok(all
        .into_iter()
        .filter(|p| {
            let name = p
                .executable
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !["chrome.exe", "chromium.exe", "ccbaga-b.exe"].contains(&name.as_str()) {
                return false;
            }
            p.args.iter().enumerate().any(|(index, arg)| {
                let path = arg.strip_prefix("--user-data-dir=").or_else(|| {
                    if arg == "--user-data-dir" {
                        p.args.get(index + 1).map(String::as_str)
                    } else {
                        None
                    }
                });
                path.is_some_and(|path| same_profile_path(Path::new(path), Path::new(profile)))
            })
        })
        .collect())
}
pub unsafe fn rf_browser_running(profile: *const c_char) -> *mut c_char {
    response(profile_processes(input(profile)).map(|ps| json!({"running":!ps.is_empty()})))
}
unsafe extern "system" fn close_window(window: HWND, data: LPARAM) -> windows_sys::core::BOOL {
    let mut pid = 0;
    GetWindowThreadProcessId(window, &mut pid);
    if pid == data as u32 {
        PostMessageW(window, WM_CLOSE, 0, 0);
    }
    1
}
pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    response((|| {
        for process in profile_processes(input(profile))? {
            if process.args.iter().any(|arg| arg.starts_with("--type=")) {
                continue;
            }
            let _identity = verified_process(&process, PROCESS_QUERY_LIMITED_INFORMATION)?;
            EnumWindows(Some(close_window), process.pid as LPARAM);
        }
        Ok(json!({"ok":true}))
    })())
}

const SNAPSHOT: &str = r#"
$mapPath=Join-Path $env:SystemRoot 'Globalization\Time Zone\timezoneMapping.xml'
[xml]$map=Get-Content -LiteralPath $mapPath -Raw
$zones=@($map.TimeZoneMapping.MapTZ | ForEach-Object { [string]$_.TZID } | Sort-Object -Unique)
$windowsZone=(Get-TimeZone).Id
$timezone=([Windows.Globalization.Calendar,Windows.Globalization,ContentType=WindowsRuntime]::new()).GetTimeZone()
$langs=@(Get-WinUserLanguageList | ForEach-Object { $_.LanguageTag })
$auto=$null; try { $auto=((Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\tzautoupdate' -Name Start).Start -eq 3) } catch {}
$location=$null
try {
  $device=(Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name Value).Value
  $user=(Get-ItemProperty -LiteralPath 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name Value -ErrorAction SilentlyContinue).Value
  if($device -eq 'Deny') { $location=$false } elseif($device -eq 'Allow') { $location=$true }
} catch {}
$version=(Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').CurrentBuildNumber
$proxy=''
try { $p=Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'; if($p.ProxyEnable -eq 1){$proxy=[string]$p.ProxyServer}; if($p.AutoConfigURL){$proxy='pac:'+ [string]$p.AutoConfigURL} } catch {}
try { $connection=(Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings\Connections').DefaultConnectionSettings; if($connection.Length -gt 8 -and ($connection[8] -band 8) -ne 0){$proxy='pac:auto-detect'} } catch {}
@{timezone=$timezone;windows_timezone=$windowsZone;timezones=$zones;automatic_timezone=$auto;languages=$langs;locale=(Get-Culture).Name.Replace('-','_');location_enabled=$location;accessibility=$false;platform='windows';version=('10.0.'+$version);cores=[Environment]::ProcessorCount;system_proxy=$proxy} | ConvertTo-Json -Compress -Depth 4
"#;
fn normalized_language(value: &str) -> String {
    match value {
        "zh-CN" | "zh-SG" | "zh-Hans" | "zh-Hans-CN" | "zh-Hans-SG" => "zh-Hans".into(),
        "zh-TW" | "zh-HK" | "zh-MO" | "zh-Hant" | "zh-Hant-TW" | "zh-Hant-HK" | "zh-Hant-MO" => {
            "zh-Hant".into()
        }
        s if s.starts_with("en-") || s == "pt-BR" => s.into(),
        s => s.split('-').next().unwrap_or(s).into(),
    }
}
fn snapshot() -> Result<Value, String> {
    let mut value: Value = serde_json::from_str(&powershell(SNAPSHOT, &[])?)
        .map_err(|_| "Invalid Windows system snapshot.")?;
    if let Some(languages) = value["languages"].as_array_mut() {
        for item in languages {
            if let Some(s) = item.as_str() {
                *item = json!(normalized_language(s));
            }
        }
    }
    value["system_proxy"] = json!(system_proxy(value["system_proxy"].as_str().unwrap_or("")));
    Ok(value)
}
fn system_proxy(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    // PAC/WPAD cannot be resolved by reqwest. An unsupported scheme fails before
    // opening a connection, rather than silently bypassing the browser's route.
    if value.starts_with("pac:") {
        return "unsupported-windows-proxy://automatic-configuration".into();
    }
    if value.contains('=') {
        let entries: Vec<_> = value
            .split(';')
            .filter_map(|s| s.trim().split_once('='))
            .collect();
        if let Some((_, host)) = entries
            .iter()
            .find(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
        {
            return format!("http://{host}");
        }
        if let Some((_, host)) = entries
            .iter()
            .find(|(scheme, _)| scheme.eq_ignore_ascii_case("socks"))
        {
            return format!("socks4://{host}");
        }
        // A proxy configured only for HTTP does not carry the HTTPS exit checks.
        return String::new();
    }
    if value.contains("://") {
        value.into()
    } else {
        format!("http://{value}")
    }
}
pub unsafe fn rf_browser_machine() -> *mut c_char {
    response(snapshot())
}
pub unsafe fn rf_system_read() -> *mut c_char {
    response(snapshot().map(|mut state| {
        state.as_object_mut().unwrap().remove("system_proxy");
        state
    }))
}
pub unsafe fn rf_system_open(pane: *const c_char) -> *mut c_char {
    response(open_settings(input(pane)))
}
fn public_state() -> Result<Value, String> {
    let mut state = snapshot()?;
    state.as_object_mut().unwrap().remove("system_proxy");
    Ok(state)
}
pub unsafe fn rf_system_timezone(zone: *const c_char) -> *mut c_char {
    response((|| {
        let zone = input(zone);
        let state = snapshot()?;
        if !state["timezones"]
            .as_array()
            .is_some_and(|zones| zones.iter().any(|z| z.as_str() == Some(zone)))
        {
            return Err("Invalid time zone.".into());
        }
        powershell(
            r#"
[xml]$map=Get-Content -LiteralPath (Join-Path $env:SystemRoot 'Globalization\Time Zone\timezoneMapping.xml') -Raw
$zone=@($map.TimeZoneMapping.MapTZ | Where-Object { $_.TZID -ceq $env:CCBAGA_TIMEZONE })[0]
if(!$zone){throw 'Unknown time zone'}
$tool=Join-Path $env:SystemRoot 'System32\tzutil.exe'
& $tool /s ([string]$zone.WinID)
if($LASTEXITCODE -ne 0){
  $elevated=Start-Process -FilePath $tool -ArgumentList @('/s',('"'+[string]$zone.WinID+'"')) -Verb RunAs -WindowStyle Hidden -PassThru -Wait
  if($elevated.ExitCode -ne 0){throw 'Windows denied the timezone change'}
}
if((Get-TimeZone).Id -cne [string]$zone.WinID){throw 'Timezone verification failed'}
"#,
            &[("CCBAGA_TIMEZONE", zone)],
        )?;
        Ok(
            json!({"state":public_state()?,"message":"Windows time zone applied and its Windows identifier verified. Windows maps some IANA zones to the same time zone. Automatic time zone remains controlled in Date & time Settings."}),
        )
    })())
}
fn language_tag(requested: &str) -> Option<&'static str> {
    match requested {
        "en-US" => Some("en-US"),
        "en-CA" => Some("en-CA"),
        "en-GB" => Some("en-GB"),
        "en-SG" => Some("en-SG"),
        "en-AU" => Some("en-AU"),
        "en" => Some("en-US"),
        "zh-Hans" => Some("zh-CN"),
        "zh-Hant" => Some("zh-TW"),
        "fr" => Some("fr-FR"),
        "de" => Some("de-DE"),
        "es" => Some("es-ES"),
        "it" => Some("it-IT"),
        "ja" => Some("ja-JP"),
        "ko" => Some("ko-KR"),
        _ => None,
    }
}
const SET_LANGUAGE: &str = r#"
$old=Get-WinUserLanguageList
$updated=New-WinUserLanguageList $env:CCBAGA_LANGUAGE
$requested=$updated[0].LanguageTag
$existing=@($old | Where-Object {$_.LanguageTag -ieq $requested})
if($existing.Count -gt 0){$updated.Clear();[void]$updated.Add($existing[0])}
foreach($entry in $old){if($entry.LanguageTag -ine $requested){[void]$updated.Add($entry)}}
Set-WinUserLanguageList -LanguageList $updated -Force
$actual=@(Get-WinUserLanguageList)
if(!$actual -or $actual[0].LanguageTag -ine $requested){throw 'Preferred language verification failed'}
foreach($entry in $old){if(@($actual | Where-Object {$_.LanguageTag -ieq $entry.LanguageTag}).Count -eq 0){throw 'Previous language was not preserved'}}
"#;
pub unsafe fn rf_system_language(language: *const c_char) -> *mut c_char {
    response((|| {
        let tag = language_tag(input(language)).ok_or("Invalid language.")?;
        powershell(SET_LANGUAGE, &[("CCBAGA_LANGUAGE", tag)])?;
        Ok(
            json!({"state":public_state()?,"message":"Preferred language order applied and verified; other languages were preserved. Windows display language packs and sign-out requirements are controlled in Language & region Settings."}),
        )
    })())
}
pub unsafe fn rf_system_region(language: *const c_char, locale: *const c_char) -> *mut c_char {
    response((|| {
        let requested = input(language);
        let locale = input(locale);
        if !crate::regions::get_regions()
            .iter()
            .any(|r| r.language == requested && r.locale == locale)
        {
            return Err("Invalid language/region combination.".into());
        }
        let tag = locale.replace('_', "-");
        powershell(SET_LANGUAGE, &[("CCBAGA_LANGUAGE", &tag)])?;
        powershell("Set-Culture -CultureInfo $env:CCBAGA_LOCALE; if ((Get-ItemProperty -LiteralPath 'HKCU:\\Control Panel\\International' -Name LocaleName).LocaleName -ine $env:CCBAGA_LOCALE) { throw 'Regional format verification failed' }", &[("CCBAGA_LOCALE", &tag)])
            .map_err(|e| format!("Preferred language was changed; {e}"))?;
        Ok(
            json!({"state":public_state()?,"message":"Preferred language and regional format preferences applied and verified. Existing applications may require restart; display-language packs and sign-out remain managed by Windows."}),
        )
    })())
}
pub unsafe fn rf_system_location() -> *mut c_char {
    response((|| {
        let state = public_state()?;
        if state["location_enabled"] == false {
            return Ok(
                json!({"state":state,"message":"Windows location permission is disabled in the observed system/user preference. Browser and third-party geolocation are verified separately."}),
            );
        }
        open_settings("location")?;
        Err("Windows Location Settings opened. Turn off Location services, then refresh or retry to verify. No disabled state has been claimed.".into())
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_allowlist_and_reference_do_not_accept_neighbors() {
        for target in [
            "Claude Code-credentials",
            "Claude Code-credentials-test",
            "Claude Safe Storage",
        ] {
            assert!(allowed_credential(target));
        }
        for target in [
            "Other Claude",
            "Claude Code",
            "Claude Safe Storage-extra",
            "git:https://claude.ai",
        ] {
            assert!(!allowed_credential(target));
        }
    }
    #[test]
    fn command_line_parser_preserves_windows_spaces_and_exact_cli_entry() {
        let args = parse_command_line(
            r#""C:\Program Files\nodejs\node.exe" "C:\Users\Test User\node_modules\@anthropic-ai\claude-code\cli.js" --help"#,
        );
        assert_eq!(args.len(), 3);
        let mut process = Process {
            pid: 1,
            executable: PathBuf::from(r"C:\Program Files\nodejs\node.exe"),
            args,
            started: "fixture".into(),
        };
        assert!(node_cli(&process));
        process.args[1].push_str(".bak");
        assert!(!node_cli(&process));
        process.args[1] = "--eval".into();
        assert!(!node_cli(&process));
        process.args = vec![
            "node.exe".into(),
            "--require".into(),
            r"C:\node_modules\@anthropic-ai\claude-code\cli.js".into(),
            "unrelated.js".into(),
        ];
        assert!(!node_cli(&process));
    }
    #[test]
    fn claude_process_classification_keeps_directory_and_version_boundaries() {
        let home = Path::new(r"\\?\C:\Users\Fixture");
        let local = Path::new(r"C:\Users\Fixture\AppData\Local");
        let packages = vec![PathBuf::from(
            r"D:\WindowsApps\Claude_1.2.3.0_x64__pzs8sxrjxfjjc",
        )];
        let classify = |path: &str| {
            claude_kind(
                &Process {
                    pid: 1,
                    executable: path.into(),
                    args: vec![],
                    started: "fixture".into(),
                },
                home,
                local,
                &packages,
            )
        };
        for path in [
            r"c:\users\fixture\.LOCAL\bin\CLAUDE.EXE",
            r"C:\Users\Fixture\.local\share\claude\versions\2.1.101.exe",
            r"C:\Users\Fixture\.local\share\claude\versions\2.1.101",
            r"C:\Users\Fixture\AppData\Local\Microsoft\WinGet\Packages\Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe\claude.exe",
        ] {
            assert_eq!(classify(path), Some("Claude Code"), "{path}");
        }
        for path in [
            r"C:\Users\Fixture\AppData\Local\AnthropicClaude\app-1.24012.0\claude.exe",
            r"C:\Users\Fixture\AppData\Local\AnthropicClaude\app-1.24012.0\resources\chrome_crashpad_handler.exe",
            r"D:\WindowsApps\Claude_1.2.3.0_x64__pzs8sxrjxfjjc\app\claude.exe",
        ] {
            assert_eq!(classify(path), Some("Claude Desktop"), "{path}");
        }
        for path in [
            r"C:\Other\claude.exe",
            r"C:\Users\Fixture\.local\bin\nested\claude.exe",
            r"C:\Users\Fixture\.local\share\claude\versions\2.1.101.bak",
            r"C:\Users\Fixture\.local\share\claude\versions\2.1.101\claude.exe",
            r"C:\Users\Fixture\AppData\Local\Microsoft\WinGet\Packages\Anthropic.ClaudeCodeFake_Microsoft.Winget.Source_8wekyb3d8bbwe\claude.exe",
            r"C:\Users\Fixture\AppData\Local\AnthropicClaudeOther\claude.exe",
            r"C:\Users\Fixture\AppData\Local\AnthropicClaude\data\unrelated.exe",
            r"D:\WindowsApps\Claude_1.2.3.0_x64__pzs8sxrjxfjjc-other\claude.exe",
        ] {
            assert_eq!(classify(path), None, "{path}");
        }
    }
    #[test]
    fn current_process_read_checks_owner_and_creation_identity_without_termination() {
        let mut current = processes()
            .unwrap()
            .into_iter()
            .find(|p| p.pid == std::process::id() as i32)
            .unwrap();
        assert!(!current.args.is_empty());
        assert!(verified_process(&current, PROCESS_QUERY_LIMITED_INFORMATION).is_ok());
        current.started.push('0');
        assert!(verified_process(&current, PROCESS_QUERY_LIMITED_INFORMATION).is_err());
    }
    #[test]
    fn languages_and_settings_reject_untrusted_input_before_mutating() {
        assert_eq!(normalized_language("zh-HK"), "zh-Hant");
        assert_eq!(normalized_language("zh-Hans-CN"), "zh-Hans");
        assert_eq!(normalized_language("en-CA"), "en-CA");
        assert_eq!(language_tag("en-US; bad"), None);
        assert!(open_settings("bad; start powershell").is_err());
        assert!(open_url("file:///C:/bad.exe").is_err());
    }
    #[test]
    fn proxy_selection_uses_https_route_and_refuses_automatic_bypass() {
        assert_eq!(system_proxy("localhost:7890"), "http://localhost:7890");
        assert_eq!(
            system_proxy("http=localhost:80;https=localhost:443"),
            "http://localhost:443"
        );
        assert_eq!(
            system_proxy("http=localhost:80;socks=localhost:1080"),
            "socks4://localhost:1080"
        );
        assert_eq!(system_proxy("http=localhost:80"), "");
        assert_eq!(
            system_proxy("pac:https://example.invalid/private.pac"),
            "unsupported-windows-proxy://automatic-configuration"
        );
    }
    #[test]
    fn native_file_identity_is_stable_for_a_read_only_handle() {
        let path = std::env::current_exe().unwrap();
        let first = file_identity(&path).unwrap();
        assert_eq!(first, file_identity(&path).unwrap());
        use std::os::windows::io::AsRawHandle;
        let opened = std::fs::File::open(path).unwrap();
        assert!(!opened.as_raw_handle().is_null());
    }
    #[test]
    fn profile_comparison_resolves_existing_dot_segments() {
        let actual = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let alternate = actual.join("..").join(actual.file_name().unwrap());
        assert!(same_profile_path(&actual, &alternate));
        assert!(!same_profile_path(&actual, &actual.join("unrelated")));
    }
    #[test]
    fn change_time_detects_same_length_writes_even_when_last_write_is_restored() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "ccbaga-windows-change-time-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(b"before").unwrap();
        file.sync_all().unwrap();
        let before_modified = file.metadata().unwrap().modified().unwrap();
        let before_change = file_change_time(&path).unwrap();
        let before_identity = file_identity(&path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(b"after!").unwrap();
        file.sync_all().unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(before_modified))
            .unwrap();
        drop(file);
        let after = file_change_time(&path).unwrap();
        let identity = file_identity(&path).unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(before_identity, identity);
        assert_eq!(before_modified, modified);
        assert_ne!(before_change, after);
    }
}
