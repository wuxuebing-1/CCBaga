// [INPUT]: 临时样本、指纹参数、固定清理边界与本机 HTTP 限流样本。
// [OUTPUT]: 模块共用临时目录样本，配置/缓存/链接/Windows junction/地区核验与 HTTP 429 服务备援回归。
// [POS]: 浏览器模块行为验证；不访问用户实际资料。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
fn machine() -> Machine {
    Machine {
        platform: "macos".into(),
        version: "27.2.0".into(),
        cores: 10,
        timezone: "America/New_York".into(),
        languages: vec!["en-US".into()],
        locale: "en_US".into(),
        location_enabled: Some(false),
        timezones: vec!["America/New_York".into()],
        system_proxy: String::new(),
    }
}
pub(super) fn sample() -> PathBuf {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("rf-browser-{}", server::nonce().unwrap()));
    fs::create_dir(&root).unwrap();
    root
}
#[test]
fn proxy_and_real_hardware() {
    let mut c = Config::default();
    for bad in [
        "http://a:8080/--disable-web-security",
        "http://user:pass@a:8080",
        "file:///tmp/a",
        "http://a:8080?q=x",
    ] {
        assert!(config::proxy(bad).is_err());
    }
    c.proxy = "socks5h://127.0.0.1:7898".into();
    let a = config::arguments(
        &c,
        &machine(),
        Path::new("/profile"),
        "http://127.0.0.1/page",
        "148.0.7778.215",
    );
    assert!(a.contains(&"--fingerprint-brand-version=148.0.7778.215".into()));
    assert!(a.contains(&"--profile-directory=Default".into()));
    assert!(a.contains(&"--fingerprint-platform-version=27.2.0".into()));
    assert!(a.contains(&"--fingerprint-hardware-concurrency=10".into()));
    assert!(a.iter().any(|x| x.starts_with("--host-resolver-rules=")));
    assert!(!a
        .iter()
        .any(|x| x.contains("load-extension") || x.contains("disable-web-security")));
}
#[test]
fn cache_isolation_and_direct_delete() {
    let root = sample();
    for name in [
        "Default/Cache/data",
        "Default/Network/Cookies",
        "Default/Bookmarks",
        "Default/Login Data",
    ] {
        let p = root.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"fixture").unwrap();
    }
    let mut log = Vec::new();
    assert_eq!(
        profile::clear(&root, false, |n, d| log.push((n.to_string(), d))).unwrap(),
        1
    );
    assert_eq!(log.len(), 2);
    assert!(!root.join("Default/Cache").exists());
    assert!(root.join("Default/Network/Cookies").exists());
    profile::clear(&root, true, |_, _| {}).unwrap();
    assert!(!root.join("Default/Network/Cookies").exists());
    assert!(root.join("Default/Bookmarks").exists());
    assert!(root.join("Default/Login Data").exists());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn reject_links() {
    let root = sample();
    let outside = sample();
    fs::write(outside.join("keep"), b"keep").unwrap();
    fs::create_dir(root.join("Default")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("Default/Cache")).unwrap();
    assert!(profile::clear(&root, false, |_, _| {}).is_err());
    assert!(outside.join("keep").exists());
    assert!(server::resource(&root, "Default/Cache/keep").is_none());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
#[test]
fn reject_malformed_preferences_and_parent_traversal() {
    let root = sample();
    fs::create_dir(root.join("Default")).unwrap();
    let prefs = root.join("Default/Preferences");
    fs::write(&prefs, b"{\"intl\":\"wrong\"}").unwrap();
    assert!(profile::preferences(&root, &Config::default(), &machine()).is_err());
    assert_eq!(fs::read(&prefs).unwrap(), b"{\"intl\":\"wrong\"}");
    assert!(server::resource(&root, "../keep").is_none());
    assert!(profile::check_path(&root, &root.join("../outside")).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn windows_junction_cannot_redirect_profile_cleanup() {
    let root = sample();
    let outside = sample();
    fs::write(outside.join("keep"), b"keep").unwrap();
    fs::create_dir(root.join("Default")).unwrap();
    let link = root.join("Default/Cache");
    crate::windows::powershell(
        "New-Item -ItemType Junction -Path $env:CCBAGA_TEST_LINK -Target $env:CCBAGA_TEST_TARGET | Out-Null",
        &[("CCBAGA_TEST_LINK", &link.to_string_lossy()), ("CCBAGA_TEST_TARGET", &outside.to_string_lossy())],
    ).unwrap();
    assert!(profile::check_path(&root, &link.join("keep")).is_err());
    assert!(profile::clear(&root, false, |_, _| {}).is_err());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
    fs::remove_dir(&link).unwrap();
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn saved_settings_and_launch_arguments_can_be_replaced_without_backups() {
    let root = sample();
    let mut config = Config::default();
    config::save(&root, &config).unwrap();
    write_launch_args(&root, &config, &machine(), &root.join("profile"), "148").unwrap();
    config.seed = 37;
    config::save(&root, &config).unwrap();
    write_launch_args(&root, &config, &machine(), &root.join("profile"), "148").unwrap();
    assert_eq!(config::load(&root).unwrap().seed, 37);
    assert!(fs::read_to_string(root.join("launch.args"))
        .unwrap()
        .contains("--fingerprint=37\n"));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn preferences_preserve_assets_and_block_both_geolocation_modes() {
    let root = sample();
    fs::create_dir(root.join("Default")).unwrap();
    fs::write(root.join("Default/Preferences"),br#"{"bookmarks":{"keep":true},"profile":{"content_settings":{"exceptions":{"geolocation_with_options":{"site":{"setting":1}}}}}}"#).unwrap();
    profile::preferences(&root, &Config::default(), &machine()).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("Default/Preferences")).unwrap()).unwrap();
    assert_eq!(value["bookmarks"]["keep"], true);
    assert_eq!(
        value["profile"]["default_content_setting_values"]["geolocation_with_options"],
        serde_json::json!({"approximate":2,"precise":2})
    );
    assert_eq!(
        value["profile"]["content_settings"]["exceptions"]["geolocation_with_options"],
        serde_json::json!({})
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn observations_required() {
    let checks = network::checks(&machine(), &Config::default(), None, None, "148");
    assert_eq!(checks.iter().filter(|x| x.status == "pending").count(), 9);
    assert!(
        network::parse(&serde_json::json!({"success":true,"ip":"fake","country_code":"US"}))
            .is_err()
    );
}
#[test]
fn country_checks_follow_region_instead_of_assuming_america() {
    let mut machine = machine();
    machine.timezone = "Asia/Tokyo".into();
    machine.languages = vec!["ja".into()];
    machine.locale = "ja_JP".into();
    let mut config = Config::default();
    config.follow_machine = false;
    config.language = "ja".into();
    config.timezone = "Asia/Tokyo".into();
    let mut report = network::Report {
        language: "ja".into(),
        languages: vec!["ja".into()],
        timezone: "Asia/Tokyo".into(),
        platform: "MacIntel".into(),
        cores: 10,
        user_agent: "fixture".into(),
        platform_version: machine.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: Some(network::Exit {
            ip: "203.0.113.1".into(),
            country: "JP".into(),
            city: "Tokyo".into(),
            timezone: "Asia/Tokyo".into(),
            isp: "fixture".into(),
        }),
    };
    let checks = network::checks(&machine, &config, Some(&report), None, "148");
    assert_eq!(checks[0].status, "ok");
    assert_eq!(checks[3].status, "ok");
    report.exit.as_mut().unwrap().country = "US".into();
    assert_eq!(
        network::checks(&machine, &config, Some(&report), None, "148")[0].status,
        "mismatch"
    );
    assert_eq!(
        crate::regions::matching("ja", "Europe/Paris").map(|r| r.country),
        None
    );
}

#[test]
fn rate_limited_service_falls_back_without_changing_route() {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for status in ["429 Too Many Requests", "200 OK"] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            let body = r#"{"ip":"203.0.113.2","country_code":"JP","timezone":"Asia/Tokyo","org":"fixture"}"#;
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let urls = [
        format!("http://{address}/primary"),
        format!("http://{address}/backup"),
    ];
    let exit =
        tauri::async_runtime::block_on(network::probe_urls("", &[&urls[0], &urls[1]])).unwrap();
    assert_eq!(exit.ip, "203.0.113.2");
    assert_eq!(exit.timezone, "Asia/Tokyo");
    assert!(network::parse(
        &serde_json::json!({"error":true,"ip":"203.0.113.2","country_code":"US"})
    )
    .is_err());
    server.join().unwrap();
}

#[test]
fn linux_platform_and_unknown_location_cannot_be_misreported_as_mac_or_disabled() {
    let mut m = machine();
    m.platform = "linux".into();
    m.version = "6.18.0".into();
    m.location_enabled = None;
    let args = config::arguments(
        &Config::default(),
        &m,
        Path::new("/profile"),
        "http://localhost",
        "148",
    );
    assert!(args.contains(&"--fingerprint-platform=linux".into()));
    let report = network::Report {
        language: "en-US".into(),
        languages: vec!["en-US".into()],
        timezone: m.timezone.clone(),
        platform: "Linux x86_64".into(),
        cores: m.cores,
        user_agent: "fixture".into(),
        platform_version: String::new(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        error: String::new(),
        exit: None,
    };
    let checks = network::checks(&m, &Config::default(), Some(&report), None, "148");
    assert_eq!(
        checks
            .iter()
            .find(|c| c.title == "真实设备与 CPU")
            .unwrap()
            .status,
        "ok"
    );
    assert_ne!(
        checks
            .iter()
            .find(|c| c.title == "地理位置关闭")
            .unwrap()
            .status,
        "ok"
    );
}

#[test]
fn machine_proxy_credentials_never_leave_native_code() {
    let mut m = machine();
    m.system_proxy = "http://private-user:private-password@localhost:8080".into();
    let value = serde_json::to_string(&m).unwrap();
    assert!(!value.contains("private-password"));
    assert!(!value.contains("system_proxy"));
}

#[test]
fn windows_hardware_check_requires_real_windows_platform_version_and_cpu() {
    let mut m = machine();
    m.platform = "windows".into();
    m.version = "10.0.26100".into();
    let args = config::arguments(
        &Config::default(),
        &m,
        Path::new("C:\\profile"),
        "http://localhost",
        "148",
    );
    assert!(args.contains(&"--fingerprint-platform=windows".into()));
    assert!(args.contains(&"--fingerprint-platform-version=10.0.26100".into()));
    let mut report = network::Report {
        language: "en-US".into(),
        languages: vec!["en-US".into()],
        timezone: m.timezone.clone(),
        platform: "Win32".into(),
        cores: m.cores,
        user_agent: "fixture".into(),
        platform_version: m.version.clone(),
        browser_version: "148".into(),
        geolocation: "denied".into(),
        exit: None,
        error: String::new(),
    };
    let status = |r: &network::Report| {
        network::checks(&m, &Config::default(), Some(r), None, "148")
            .into_iter()
            .find(|c| c.title == "真实设备与 CPU")
            .unwrap()
            .status
    };
    assert_eq!(status(&report), "ok");
    report.platform = "MacIntel".into();
    assert_eq!(status(&report), "mismatch");
    report.platform = "Win32".into();
    report.platform_version = "15.0.0".into();
    assert_eq!(status(&report), "mismatch");
    report.platform_version = m.version.clone();
    report.cores += 1;
    assert_eq!(status(&report), "mismatch");
}
