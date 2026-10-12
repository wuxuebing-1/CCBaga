// [INPUT]: 固定 SHA-256 Windows Fingerprint Chromium、独立资料、单实例宿主和真实 Windows 默认应用状态。
// [OUTPUT]: A/B 双向启动、回环会话重连、按用户注册浏览器与 Windows 设置确认入口。
// [POS]: browser 的 Windows 边界；只转发网址/绝对文件路径，不接受 Chromium 开关，不写 UserChoice。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
const WINDOWS_BROWSER_PROG_ID: &str = "CCBaga.Browser";

fn binary(engine: &Path) -> PathBuf {
    engine.join("chrome.exe")
}
fn packaged_engine(resources: &Path) -> PathBuf {
    resources.join("browser-runtime/WindowsBrowser")
}
fn engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resources = app.path().resource_dir().map_err(|_| "资源目录不可用。")?;
    let packaged = packaged_engine(&resources);
    if binary(&packaged).is_file() || !cfg!(debug_assertions) {
        return Ok(packaged);
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../browser-runtime/WindowsBrowser"))
}
fn prepare_engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let engine = engine(app)?;
    if !binary(&engine).is_file() || version(&engine).is_empty() {
        return Err("Windows 指纹浏览器引擎缺失或不完整。请重新安装完整 CCBaga Windows 安装包；开发构建先运行 python scripts/fetch-browser.py。".into());
    }
    Ok(engine)
}
fn version(engine: &Path) -> String {
    fs::read_to_string(engine.join("version.txt"))
        .unwrap_or_default()
        .trim()
        .into()
}
fn register(_: &Path) -> Result<(), String> {
    // Opening either application must not change the user's default associations.
    Ok(())
}
fn browser_targets(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .skip(1)
        .filter(|value| {
            !value.starts_with('-')
                && !value.contains(['\0', '\n', '\r'])
                && (reqwest::Url::parse(value)
                    .map(|url| matches!(url.scheme(), "http" | "https" | "file"))
                    .unwrap_or(false)
                    || Path::new(value).is_absolute())
        })
        .collect()
}
pub fn open_urls(app: tauri::AppHandle, args: Vec<String>) {
    let urls = browser_targets(args);
    std::thread::spawn(move || {
        if let Err(error) = start_windows_companion(&app, &urls) {
            eprintln!("CCBaga-B: {error}");
        }
    });
}
pub fn companion(app: tauri::AppHandle) {
    open_urls(app, std::env::args().collect());
}
fn start_windows_companion(app: &tauri::AppHandle, urls: &[String]) -> Result<(), String> {
    let state = app.state::<Browser>();
    let _guard = state.gate.lock().unwrap();
    let engine = prepare_engine(app)?;
    let root = root(app)?;
    let profile = root.join("profile");
    profile::check_path(&root, &profile)?;
    let config = config::load(&root)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    if !profile::running(&profile)? {
        profile::preferences(&profile, &config, &machine)?;
    }
    let version = version(&engine);
    write_launch_args(&root, &config, &machine, &profile, &version)?;
    // Always establish a new report page. An already-running Chromium hands this
    // URL to the same profile process, reconnecting it after a host restart.
    let page = open_session(app, &config, &machine, version.clone())?;
    let mut child = Command::new(binary(&engine))
        .args(config::arguments(
            &config, &machine, &profile, &page, &version,
        ))
        .args(urls)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("无法启动 Windows 指纹浏览器：{error}"))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn default_read() -> Result<DefaultBrowser, String> {
    let data = crate::windows::powershell(
        r#"
function Choice($scheme) {
    $path = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\' + $scheme + '\UserChoice'
    if (Test-Path -LiteralPath $path) { return [string](Get-ItemProperty -LiteralPath $path -Name ProgId -ErrorAction SilentlyContinue).ProgId }
    return ''
}
@{http=(Choice 'http');https=(Choice 'https')} | ConvertTo-Json -Compress
"#,
        &[],
    )?;
    let values: serde_json::Value =
        serde_json::from_str(data.trim()).map_err(|_| "Windows 默认应用信息格式无效。")?;
    let http = values["http"].as_str().unwrap_or("").to_owned();
    let https = values["https"].as_str().unwrap_or("").to_owned();
    let host = std::env::current_exe().map_err(|_| "无法定位 CCBaga-A。")?;
    let installed = host
        .parent()
        .map(|parent| binary(&packaged_engine(parent)).is_file())
        .unwrap_or(false)
        || (cfg!(debug_assertions)
            && binary(
                &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../browser-runtime/WindowsBrowser"),
            )
            .is_file());
    Ok(DefaultBrowser {
        is_default: installed
            && http == WINDOWS_BROWSER_PROG_ID
            && https == WINDOWS_BROWSER_PROG_ID,
        http,
        https,
        installed,
        app: host.to_string_lossy().into(),
    })
}

#[tauri::command]
pub async fn browser_set_default(app: tauri::AppHandle) -> Result<DefaultBrowser, String> {
    run(move || {
        prepare_engine(&app)?;
        let executable = std::env::current_exe().map_err(|_| "无法定位 CCBaga-A。")?;
        let executable = executable.to_string_lossy().to_string();
        if executable.contains(['"', '\n', '\r', '\0']) {
            return Err("应用路径无效。".into());
        }
        // Values come through environment variables, never interpolated into a
        // PowerShell command. No UserChoice/hash write or silent default change.
        crate::windows::powershell(r#"
$class = 'Software\Classes\CCBaga.Browser'
$client = 'Software\Clients\StartMenuInternet\CCBaga-B'
function Put($path, $name, $value) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($path)
    try { $key.SetValue($name, $value, [Microsoft.Win32.RegistryValueKind]::String) } finally { $key.Dispose() }
}
$command = '"' + $env:CCBAGA_BROWSER_HOST + '" --browser "%1"'
$icon = '"' + $env:CCBAGA_BROWSER_HOST + '",0'
Put $class '' 'CCBaga-B web page'
Put $class 'URL Protocol' ''
Put ($class + '\DefaultIcon') '' $icon
Put ($class + '\shell\open\command') '' $command
Put $client '' 'CCBaga-B'
Put ($client + '\DefaultIcon') '' $icon
Put ($client + '\shell\open\command') '' ('"' + $env:CCBAGA_BROWSER_HOST + '" --browser')
Put ($client + '\Capabilities') 'ApplicationName' 'CCBaga-B'
Put ($client + '\Capabilities') 'ApplicationDescription' 'CCBaga independent fingerprint Chromium browser'
Put ($client + '\Capabilities') 'ApplicationIcon' $icon
foreach ($scheme in @('http','https')) { Put ($client + '\Capabilities\URLAssociations') $scheme 'CCBaga.Browser' }
foreach ($extension in @('.htm','.html')) { Put ($client + '\Capabilities\FileAssociations') $extension 'CCBaga.Browser' }
Put 'Software\RegisteredApplications' 'CCBaga-B' ($client + '\Capabilities')
"#, &[("CCBAGA_BROWSER_HOST", &executable)])?;
        crate::windows::open_settings("default")?;
        // The user chooses in Windows Settings; report only the current choice.
        default_read()
    }).await
}

#[test]
fn windows_external_targets_cannot_supply_browser_flags() {
    let targets = browser_targets(
        vec![
            "CCBaga-A.exe",
            "--browser",
            "--user-data-dir=C:\\other",
            "javascript:alert(1)",
            "https://example.com/a?b=1",
            "C:\\documents\\hello world.html",
            "file:///C:/test.html",
            "https://example.com/\n--disable-web-security",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    );
    assert_eq!(
        targets,
        vec![
            "https://example.com/a?b=1",
            "C:\\documents\\hello world.html",
            "file:///C:/test.html"
        ]
    );
}

#[test]
fn windows_packaged_engine_stays_inside_resource_directory() {
    let resources = Path::new("C:\\Program Files\\CCBaga-A");
    assert_eq!(
        binary(&packaged_engine(resources)),
        resources.join("browser-runtime/WindowsBrowser/chrome.exe")
    );
}
