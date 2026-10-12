// [INPUT]: Skill 的 providers.json 固定配置、资源编号与各平台系统 URL 打开工具。
// [OUTPUT]: 两个原始供应商地址的 URL 组装及仓库 Skill 安装入口。
// [POS]: 应用网络页唯一原生接口；不读取密钥、不执行 SSH、不接收任意 URL。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
#[cfg(not(target_os = "windows"))]
use std::process::Command;
fn provider_url(provider: &str) -> Result<String, String> {
    if provider == "skill" {
        return Ok(
            "https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation".into(),
        );
    }
    if !["webshare", "dmit"].contains(&provider) {
        return Err("无效的供应商入口。".into());
    }
    let configured: serde_json::Value =
        serde_json::from_str(include_str!("../../skills/ccbaga-network/providers.json"))
            .map_err(|_| "无效的供应商入口。")?;
    let settings = &configured[provider];
    let mut url = reqwest::Url::parse(settings["base"].as_str().ok_or("无效的供应商入口。")?)
        .map_err(|_| "无效的供应商入口。")?;
    url.query_pairs_mut().append_pair(
        settings["parameter"].as_str().ok_or("无效的供应商入口。")?,
        settings["value"].as_str().ok_or("无效的供应商入口。")?,
    );
    Ok(url.to_string())
}
#[tauri::command]
pub fn network_skill_source(app: tauri::AppHandle) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        let packaged = app
            .path()
            .resource_dir()
            .map_err(|_| "无法读取网络 Skill。")?
            .join("network-skill");
        let root = if cfg!(debug_assertions) && !packaged.join("SKILL.md").is_file() {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills/ccbaga-network")
        } else {
            packaged
        };
        for relative in [
            "SKILL.md",
            "providers.json",
            "scripts/network.py",
            "scripts/windows_security.py",
            "scripts/setup-vps.sh",
        ] {
            if !root.join(relative).is_file() {
                return Err("Windows 网络 Skill 不完整，请重新安装完整安装包。".into());
            }
        }
        Ok(Some(root.to_string_lossy().into_owned()))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(None)
    }
}
#[tauri::command]
pub async fn network_open_resource(provider: String) -> Result<(), String> {
    let url = provider_url(&provider)?;
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            crate::windows::open_url(&url)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let status = Command::new(if cfg!(target_os = "linux") {
                "xdg-open"
            } else {
                "/usr/bin/open"
            })
            .arg(url)
            .status()
            .map_err(|_| "无法打开供应商网站。")?;
            if status.success() {
                Ok(())
            } else {
                Err("无法打开供应商网站。".into())
            }
        }
    })
    .await
    .map_err(|_| "无法打开供应商网站。".to_string())?
}
#[test]
fn fixed_resources_preserve_original_destinations() {
    assert_eq!(
        provider_url("dmit").unwrap(),
        "https://www.dmit.io/aff.php?aff=28800"
    );
    assert_eq!(
        provider_url("webshare").unwrap(),
        "https://www.webshare.io/?referral_code=qlm1mt9xibc9"
    );
    assert_eq!(
        provider_url("skill").unwrap(),
        "https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation"
    );
    assert!(provider_url("https://example.com").is_err());
}
