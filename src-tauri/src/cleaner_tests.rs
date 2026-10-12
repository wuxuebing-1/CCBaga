// [INPUT]: 隔离临时目录（含 Windows 精确 MSIX 包族）、脱敏配置、清理事务与当前用户官方原生 CLI 版本路径。
// [OUTPUT]: 白名单、文件夹范围、配置保留、组织检索、直接删除、快照保护、真实日志次序与原生版本/可丢弃 Node CLI 进程的识别/终止/隔离验证。
// [POS]: 仅操作测试数据；Windows fixture 显式设为当前用户所有以兼容提升权限的 CI，保留生产所有者检查；外部 Node 参数使用普通绝对路径，不访问现有 Claude 数据。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;

#[cfg(windows)]
#[test]
fn windows_msix_cleanup_uses_exact_package_and_preserves_configuration() {
    let f = Fixture::new();
    let desktop = "AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/Claude";
    for (name, text) in [
        ("Cache/data", "cache"),
        ("Network/Cookies", "login"),
        (
            "local_sessions/session.json",
            r#"{"organization":"org-fixture123"}"#,
        ),
        (
            "conversations/chat.json",
            r#"{"organization":"org-fixture123"}"#,
        ),
        ("claude_desktop_config.json", "MCP settings must stay"),
        ("Claude Extensions/keep", "extension must stay"),
    ] {
        f.file(&format!("{desktop}/{name}"), text);
    }
    let neighbor = "AppData/Local/Packages/Claude_unrelated/LocalCache/Roaming/Claude/local_sessions/keep.json";
    f.file(neighbor, r#"{"organization":"org-fixture123"}"#);
    let (matches, complete) = targets::org_files(&f.0, "org-fixture123").unwrap();
    assert!(complete);
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().all(|p| p.starts_with(f.0.join(desktop))));
    let plan = collect(&f.0, None).unwrap();
    assert_eq!(plan.items.len(), 4);
    assert_eq!(
        plan.items
            .iter()
            .filter(|item| item.group == "history")
            .count(),
        2
    );
    let result = perform_items(
        &f.0,
        &plan.items,
        &Cleaner::default(),
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(result.completed, "{:?}", result.errors);
    for name in ["Cache", "Network", "local_sessions", "conversations"] {
        assert!(!f.0.join(desktop).join(name).exists());
    }
    for name in ["claude_desktop_config.json", "Claude Extensions/keep"] {
        assert!(f.0.join(desktop).join(name).exists());
    }
    assert!(f.0.join(neighbor).exists());
}

#[cfg(windows)]
#[test]
fn windows_cleanup_preserves_mcp_skills_and_outside_data() {
    let f = Fixture::new();
    for (path, text) in [
        ("AppData/Roaming/Claude/Cache/data", "cache"),
        ("AppData/Roaming/Claude/Network/Cookies", "login"),
        (
            "AppData/Roaming/Claude/claude_desktop_config.json",
            "MCP settings",
        ),
        ("AppData/Roaming/Claude/Claude Extensions/keep", "extension"),
        (".claude/skills/keep/SKILL.md", "skill"),
        (
            ".claude/settings.json",
            r#"{"env":{"HTTPS_PROXY":"secret"},"mcpServers":{"keep":{}},"permissions":{"allow":["Read"]}}"#,
        ),
        (".claude/backups/settings.old", "old"),
        ("project/source.ts", "source"),
    ] {
        f.file(path, text);
    }
    let scan = collect(&f.0, None).unwrap();
    assert_eq!(scan.items.len(), 4);
    let result = perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(result.completed, "{:?}", result.errors);
    for path in [
        "AppData/Roaming/Claude/claude_desktop_config.json",
        "AppData/Roaming/Claude/Claude Extensions/keep",
        ".claude/skills/keep/SKILL.md",
        "project/source.ts",
    ] {
        assert!(f.0.join(path).exists(), "{path}");
    }
    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(f.0.join(".claude/settings.json")).unwrap())
            .unwrap();
    assert!(settings["env"].get("HTTPS_PROXY").is_none());
    assert!(settings["mcpServers"]["keep"].is_object());
    assert_eq!(settings["permissions"]["allow"][0], "Read");
}

#[cfg(windows)]
#[test]
fn windows_junction_cannot_redirect_a_scan_or_stale_transaction() {
    let f = Fixture::new();
    let outside = Fixture::new();
    outside.file("cache/data", "must stay");
    let junction = f.0.join(".claude");
    let status = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside.0)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(collect(&f.0, None).unwrap().items.is_empty());
    assert!(folder::validate(&junction).is_err());
    assert!(folder::validate(&junction.join("cache")).is_err());
    fs::remove_dir(&junction).unwrap();
    f.file(".claude/cache/data", "inside");
    let scan = collect(&f.0, None).unwrap();
    fs::rename(f.0.join(".claude"), f.0.join("old-claude")).unwrap();
    assert!(std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside.0)
        .output()
        .unwrap()
        .status
        .success());
    assert!(perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || Ok(false),
        &Reporter::channel(None)
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(outside.0.join("cache/data")).unwrap(),
        "must stay"
    );
    fs::remove_dir(junction).unwrap();
}

#[test]
fn powershell_cleanup_does_not_evaluate_or_remove_compound_commands() {
    let source = "$env:HTTPS_PROXY = 'secret'\r\n$env:ANTHROPIC_API_KEY = 'value'; Write-Host keep\r\n$env:KEEP = 'yes'\r\n$env:ALL_PROXY = $(Get-Secret)\r\n";
    let (updated, keys) = config::transform(source, config::EditKind::PowerShell).unwrap();
    assert_eq!(keys, vec!["$env:HTTPS_PROXY"]);
    assert!(!updated.contains("'secret'"));
    assert!(updated.contains("Write-Host keep"));
    assert!(updated.contains("Get-Secret"));
    assert!(updated.contains("$env:KEEP = 'yes'\r\n"));
    assert!(config::needs_manual_review(
        source,
        config::EditKind::PowerShell
    ));
    for complex in [
        "$text = @'\n$env:HTTPS_PROXY = 'example'\n'@",
        "<#\n$env:HTTPS_PROXY = 'example'\n#>",
        "$text = '\n$env:HTTPS_PROXY = ''example''\n'",
        "function Run {\n$env:HTTPS_PROXY = 'inside'\n}",
    ] {
        let (updated, names) = config::transform(complex, config::EditKind::PowerShell).unwrap();
        assert_eq!(updated, complex);
        assert!(names.is_empty());
        assert!(config::needs_manual_review(
            complex,
            config::EditKind::PowerShell
        ));
    }
}
#[cfg(unix)]
#[test]
fn folder_scope_preserves_project_assets_and_refuses_outside_targets() {
    let f = Fixture::new();
    let outside = Fixture::new();
    f.file("repo/.claude/projects/session.jsonl", "history");
    f.file("repo/.claude/cache/data", "cache");
    f.file("repo/.claude/settings.local.json", "permissions");
    f.file("repo/.claude/.credentials.json", "credential");
    f.file("repo/.claude/skills/SKILL.md", "skill");
    f.file("repo/CLAUDE.md", "instructions");
    f.file("repo/.claude.json.backup.1", "old configuration");
    f.file(
        "node_modules/ignored/.claude/projects/session.jsonl",
        "dependency",
    );
    outside.file(".claude/projects/session.jsonl", "outside");
    std::os::unix::fs::symlink(&outside.0, f.0.join("linked")).unwrap();
    let plan = folder::collect(&f.0, &Reporter::channel(None)).unwrap();
    assert_eq!(plan.items.len(), 3);
    assert_eq!(plan.folder, f.0.to_string_lossy());
    assert!(plan.items.iter().all(|i| identity(i, &f.0).is_ok()));
    assert!(plan.items.iter().all(|i| identity(i, &outside.0).is_err()));
    let ids = plan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    assert!(
        perform_items(
            &f.0,
            &selected(&plan, &ids).unwrap(),
            &Cleaner::default(),
            || Ok(false),
            &Reporter::channel(None)
        )
        .unwrap()
        .completed
    );
    for name in [
        "repo/.claude/settings.local.json",
        "repo/.claude/.credentials.json",
        "repo/.claude/skills/SKILL.md",
        "repo/CLAUDE.md",
        "node_modules/ignored/.claude/projects/session.jsonl",
    ] {
        assert!(f.0.join(name).exists());
    }
    assert!(outside.0.join(".claude/projects/session.jsonl").exists());
    assert!(folder::validate(&f.0.join("linked")).is_err());
    assert!(folder::validate(&f.0.join("repo/CLAUDE.md")).is_err());
}
#[test]
fn dropped_session_subfolder_keeps_sibling_sessions() {
    let f = Fixture::new();
    f.file(".claude/projects/a/session.jsonl", "a");
    f.file(".claude/projects/b/session.jsonl", "b");
    let root = f.0.join(".claude/projects/a");
    let plan = folder::collect(&root, &Reporter::channel(None)).unwrap();
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].path, root.to_string_lossy());
    perform_items(
        &root,
        &plan.items,
        &Cleaner::default(),
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(f.0.join(".claude/projects/b/session.jsonl").exists());
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let p = std::env::temp_dir().join(format!("rf-cleaner-test-{pid}-{id}-{unique}"));
        fs::create_dir(&p).unwrap();
        let fixture = Self(fs::canonicalize(p).unwrap());
        fixture.assert_owned(&fixture.0);
        fixture
    }
    fn file(&self, name: &str, value: &str) {
        let p = self.0.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, value).unwrap();
        // Creating a child does not inherit its parent's owner SID. Set each new
        // directory and file explicitly without touching ancestors outside this fixture.
        for path in p.ancestors().take_while(|path| path.starts_with(&self.0)) {
            self.assert_owned(path);
        }
    }
    fn assert_owned(&self, path: &Path) {
        #[cfg(windows)]
        {
            crate::windows::set_cleaner_fixture_owner(&self.0, path).unwrap_or_else(|error| {
                panic!(
                    "Fixture ownership setup failed for {}: {error}",
                    path.display()
                )
            });
            assert!(
                crate::windows::owned_by_current_user(path),
                "Fixture {} must pass the production current-user SID ownership check",
                path.display()
            );
        }
        #[cfg(not(windows))]
        let _ = path;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn discovers_supported_data_and_preserves_personal_assets() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/project/session.jsonl", "history");
    f.file(".claude/skills/skill/SKILL.md", "keep");
    f.file(".claude/plugins/plugin", "keep");
    f.file(
        ".claude/settings.json",
        r#"{"permissions":{"allow":["Read"]}}"#,
    );
    f.file(
        ".claude.json",
        r#"{"userID":"old","projects":{"/my-project":{}}}"#,
    );
    f.file("unrelated/.claude.json", r#"{"userID":"unrelated"}"#);
    let plan = collect(&f.0, None).unwrap();
    assert!(plan.items.iter().any(|i| i.group == "cache"));
    assert!(plan.items.iter().any(|i| i.group == "history"));
    assert!(plan.items.iter().any(|i| i.group == "account"));
    assert!(!plan
        .items
        .iter()
        .any(|i| ["skills", "plugins", "settings.json", "unrelated"]
            .iter()
            .any(|name| i.path.contains(name))));
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/skill/SKILL.md")).unwrap(),
        "keep"
    );
}
#[cfg(unix)]
#[test]
fn ignores_linked_targets() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("outside")).unwrap();
    std::os::unix::fs::symlink(f.0.join("outside"), f.0.join(".claude")).unwrap();
    assert!(collect(&f.0, None).unwrap().items.is_empty());
}
#[test]
fn rejects_stale_or_unknown_targets() {
    let f = Fixture::new();
    f.file(".claude.json", r#"{"userID":"old"}"#);
    let plan = collect(&f.0, None).unwrap();
    let i = &plan.items[0];
    assert!(identity(i, &f.0).is_ok());
    assert!(selected(&plan, &["unknown".into()]).is_err());
    assert_eq!(
        selected(&plan, &[i.id.clone(), i.id.clone()])
            .unwrap()
            .len(),
        1
    );
    fs::rename(f.0.join(".claude.json"), f.0.join("old")).unwrap();
    f.file(".claude.json", r#"{"userID":"old"}"#);
    assert!(identity(i, &f.0).is_err());
}
#[cfg(unix)]
#[test]
fn matches_only_current_user_and_exact_claude_processes() {
    let root = home().unwrap();
    let official = format!(
        "501 46 {}/.local/share/claude/versions/2.1.296",
        root.display()
    );
    assert_eq!(
        process_row(&official, &[], 501).unwrap().name,
        "Claude Code"
    );
    assert!(process_row("501 46 /tmp/.local/share/claude/versions/2.1.296", &[], 501).is_none());
    assert!(process_row(
        &format!(
            "501 46 {}/.local/share/claude/versions/custom",
            root.display()
        ),
        &[],
        501
    )
    .is_none());
    assert!(process_row("501 42 /tmp/claude", &[], 501).is_some());
    assert!(process_row("501 43 /tmp/claude.exe", &[], 501).is_some());
    assert!(process_row("502 42 /tmp/claude", &[], 501).is_none());
    assert!(process_row("501 42 /tmp/claude-code-vm", &[], 501).is_some());
    assert!(process_row("501 42 /tmp/.claude/worktrees/im-launch", &[], 501).is_none());
    assert!(process_row("501 42 /tmp/claude-code-vm-backup", &[], 501).is_none());
    assert!(process_row("501 1 /tmp/claude", &[], 501).is_none());
    let app = process_row(
        "501 42 /Applications/Claude.app/Contents/MacOS/Claude",
        &[42],
        501,
    )
    .unwrap();
    assert_eq!(app.name, "Claude Desktop");
    let helper = process_row("501 44 /Applications/Claude.app/Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer)", &[44], 501).unwrap();
    assert_eq!(helper.name, "Claude Helper (Renderer)");
    assert!(process_row("501 45 /usr/bin/node", &[], 501).is_none());
    assert_eq!(
        process_row("501 45 /usr/bin/node", &[45], 501)
            .unwrap()
            .name,
        "Claude Code"
    );
}
#[cfg(unix)]
#[test]
fn ends_only_the_captured_disposable_process() {
    let f = Fixture::new();
    let executable = f.0.join("claude");
    f.file(
        "process.c",
        "#include <stdio.h>\n#include <unistd.h>\nint main(void) { puts(\"READY\"); fflush(stdout); sleep(30); return 0; }\n",
    );
    // ----- 自编译样本并等待启动握手；不复制带有平台身份的系统二进制 -----
    let compilation = std::process::Command::new("cc")
        .arg(f.0.join("process.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "临时进程编译失败：{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let mut child = std::process::Command::new(&executable)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut ready,
    )
    .unwrap();
    assert_eq!(ready, "READY\n", "测试样本必须完成启动后再验证进程边界");
    let target = ClaudeProcess {
        pid: child.id() as i32,
        name: "claude".into(),
        executable: executable.to_string_lossy().into_owned(),
    };
    let mismatched = ClaudeProcess {
        executable: "/wrong/claude".into(),
        ..target.clone()
    };
    signal_same_process(&mismatched, 9).unwrap();
    assert!(child.try_wait().unwrap().is_none());
    signal_same_process(&target, 9).unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
}

#[test]
fn nested_file_changes_invalidate_directory_snapshot() {
    let f = Fixture::new();
    f.file(".claude/cache/nested/file", "before");
    let scan = collect(&f.0, None).unwrap();
    let item = scan.items.iter().find(|i| i.group == "cache").unwrap();
    assert!(identity(item, &f.0).is_ok());
    f.file(".claude/cache/nested/file", "changed");
    assert!(identity(item, &f.0).is_err());
}
#[cfg(target_os = "macos")]
#[test]
fn selected_parent_suppresses_child_and_credentials_are_never_paths() {
    let f = Fixture::new();
    f.file("Library/Application Support/Claude/Cache/file", "cache");
    let scan = collect(&f.0, None).unwrap();
    let all = selected(
        &scan,
        &scan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].path.ends_with("/Claude"));
}
#[test]
fn account_reset_preserves_projects_and_mcp_and_removes_only_known_account_keys() {
    let original = r#"{"oauthAccount":{"token":"test-only"},"userID":"old","machineID":"old","projects":{"/source":{}},"mcpServers":{"server":{}},"unknown":9}"#;
    let (updated, keys) = config::transform(original, config::EditKind::Account).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value.get("oauthAccount").is_none());
    assert!(value.get("userID").is_none());
    assert!(value["projects"]["/source"].is_object());
    assert!(value["mcpServers"]["server"].is_object());
    assert_eq!(value["unknown"], 9);
    assert!(!keys.join(" ").contains("test-only"));
}
#[test]
fn settings_cleanup_preserves_telemetry_hooks_permissions_and_unrelated_secrets() {
    let original = r#"{"env":{"ANTHROPIC_API_KEY":"test-only","HTTPS_PROXY":"test-proxy","DISABLE_TELEMETRY":"1","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1","OTHER_SECRET":"keep"},"hooks":{"Stop":[]},"permissions":{"allow":["Read"]},"model":"keep"}"#;
    let (updated, _) = config::transform(original, config::EditKind::Settings).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value["env"].get("ANTHROPIC_API_KEY").is_none());
    assert_eq!(value["env"]["DISABLE_TELEMETRY"], "1");
    assert_eq!(value["env"]["OTHER_SECRET"], "keep");
    assert_eq!(value["hooks"]["Stop"], serde_json::json!([]));
    assert_eq!(value["model"], "keep");
    assert!(config::transform("{invalid", config::EditKind::Settings).is_err());
    assert!(config::transform(r#"{"env":[]}"#, config::EditKind::Settings).is_err());
}
#[test]
fn shell_and_git_cleanups_do_not_remove_unrelated_commands() {
    let shell = "# HTTPS_PROXY=comment\nexport HTTPS_PROXY='test-only'\nexport ANTHROPIC_API_KEY='test-only'\nexport HTTPS_PROXY=foo; echo keep\nexport KEEP=1\necho proxy\n";
    let (updated, keys) = config::transform(shell, config::EditKind::Shell).unwrap();
    assert_eq!(keys, vec!["ANTHROPIC_API_KEY", "HTTPS_PROXY"]);
    assert!(updated.contains("HTTPS_PROXY=comment"));
    assert!(updated.contains("foo; echo keep"));
    assert!(updated.contains("export KEEP=1"));
    let git = "[http]\n proxy = test-only\n sslVerify = true\n[remote \"origin\"]\n proxy = keep\n";
    let (updated, _) = config::transform(git, config::EditKind::Git).unwrap();
    assert!(!updated.contains("test-only"));
    assert!(updated.contains("sslVerify = true"));
    assert!(updated.contains("proxy = keep"));
}
#[cfg(unix)]
#[test]
fn organization_search_stays_in_claude_sessions_and_does_not_follow_links() {
    let f = Fixture::new();
    let org = "org-12345678";
    f.file(
        ".claude/projects/a/match.jsonl",
        r#"{"org_id":"org-12345678"}"#,
    );
    f.file(
        ".claude/projects/a/other.jsonl",
        r#"{"org_id":"org-123456789"}"#,
    );
    f.file("source/private.json", r#"{"org_id":"org-12345678"}"#);
    std::os::unix::fs::symlink(f.0.join("source"), f.0.join(".claude/projects/link")).unwrap();
    let (files, complete) = targets::org_files(&f.0, org).unwrap();
    assert!(complete);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("match.jsonl"));
    assert!(targets::org_files(&f.0, "../source").is_err());
}
#[cfg(unix)]
#[test]
fn direct_cleanup_removes_selected_data_and_backups_without_copies() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/a/history.jsonl", "conversation");
    f.file(".claude/skills/keep/SKILL.md", "skill");
    f.file(".claude/plugins/keep", "plugin");
    f.file("ClaudeBackups/requirement-french-old/data", "old backup");
    f.file(".claude/backups/.claude.json.backup.1", "old configuration");
    f.file(".claude.json.backup.2", "old configuration");
    f.file(
        ".claude.json",
        r#"{"userID":"old","mcpServers":{"keep":{}}}"#,
    );
    f.file(".npmrc", "proxy=test-only\nregistry=keep\n");
    f.file("outside-secret", "outside");
    std::os::unix::fs::symlink(f.0.join("outside-secret"), f.0.join(".claude/cache/link")).unwrap();
    let scan = collect(&f.0, None).unwrap();
    assert_eq!(
        scan.items.iter().filter(|i| i.group == "backups").count(),
        3
    );
    let items = selected(
        &scan,
        &scan.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    let reporter = Reporter::channel(None);
    let result = perform_items(&f.0, &items, &Cleaner::default(), || Ok(false), &reporter).unwrap();
    assert!(result.completed);
    assert_eq!(result.cleaned, items.len());
    for path in [
        ".claude/cache",
        ".claude/projects",
        "ClaudeBackups",
        ".claude/backups",
        ".claude.json.backup.2",
    ] {
        assert!(!f.0.join(path).exists());
    }
    assert_eq!(
        fs::read_to_string(f.0.join("outside-secret")).unwrap(),
        "outside"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/keep/SKILL.md")).unwrap(),
        "skill"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/plugins/keep")).unwrap(),
        "plugin"
    );
    assert_eq!(
        fs::read_to_string(f.0.join(".npmrc")).unwrap(),
        "registry=keep\n"
    );
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(f.0.join(".claude.json")).unwrap()).unwrap();
    assert!(value.get("userID").is_none());
    assert!(value["mcpServers"]["keep"].is_object());
    let events = reporter.events();
    assert!(!events
        .iter()
        .any(|e| e.stage() == "backup" || e.stage() == "restore"));
    let success_counts: Vec<_> = events
        .iter()
        .filter(|e| e.message().starts_with("已清理："))
        .map(|e| e.current())
        .collect();
    assert_eq!(success_counts, (1..=items.len()).collect::<Vec<_>>());
    assert!(!events
        .iter()
        .any(|e| e.message().contains("test-only") || e.message().contains("conversation")));
    assert!(collect(&f.0, None).unwrap().items.is_empty());
}
#[test]
fn interrupted_cleanup_reports_partial_and_keeps_remaining_data() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/a/history.jsonl", "history");
    let scan = collect(&f.0, None).unwrap();
    let calls = std::cell::Cell::new(0);
    let reporter = Reporter::channel(None);
    let result = perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || {
            let count = calls.get();
            calls.set(count + 1);
            Ok(count > 0)
        },
        &reporter,
    )
    .unwrap();
    let events = reporter.events();
    let failed = events
        .iter()
        .position(|e| e.stage() == "clean" && e.status() == "error")
        .unwrap();
    assert_eq!(events[failed].current(), 1);
    assert!(!events[failed + 1..]
        .iter()
        .any(|e| e.status() == "running" && e.stage() == "clean"));
    assert!(!result.completed);
    assert_eq!(result.cleaned, 1);
    assert_eq!(result.errors.len(), 1);
    assert!(f.0.join(".claude/projects/a/history.jsonl").exists());
    assert!(!f.0.join("ClaudeBackups").exists());
}
#[test]
fn stale_snapshot_stops_before_any_delete() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "keep");
    let scan = collect(&f.0, None).unwrap();
    f.file(".claude/cache/data", "new");
    let reporter = Reporter::channel(None);
    let error = perform_items(
        &f.0,
        &scan.items,
        &Cleaner::default(),
        || Ok(false),
        &reporter,
    )
    .err()
    .unwrap();
    reporter.fail(&error);
    assert_eq!(reporter.events().last().unwrap().stage(), "validate");
    assert!(!reporter.events().iter().any(|e| e.stage() == "clean"));
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/cache/data")).unwrap(),
        "new"
    );
}

#[test]
fn node_claude_entry_is_identified_and_unrelated_node_is_preserved() {
    struct Children(Vec<std::process::Child>);
    impl Drop for Children {
        fn drop(&mut self) {
            for child in &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let f = Fixture::new();
    f.file(
        "node_modules/@anthropic-ai/claude-code/cli.js",
        "setInterval(()=>{},1000)",
    );
    f.file("other.js", "setInterval(()=>{},1000)");
    // Node 22's CommonJS entry resolver rejects Windows verbatim (\\?\) paths.
    // Keep canonical paths for native safety checks; only these child arguments
    // use conventional absolute paths, including UNC paths when applicable.
    let node_path = |path: PathBuf| -> PathBuf {
        #[cfg(windows)]
        {
            use std::path::{Component, Prefix};
            let mut parts = path.components();
            if let Some(Component::Prefix(prefix)) = parts.next() {
                let conventional = match prefix.kind() {
                    Prefix::VerbatimDisk(letter) => {
                        Some(PathBuf::from(format!("{}:\\", letter as char)))
                    }
                    Prefix::VerbatimUNC(server, share) => {
                        let mut value = std::ffi::OsString::from(r"\\");
                        value.push(server);
                        value.push(r"\");
                        value.push(share);
                        Some(PathBuf::from(value))
                    }
                    _ => None,
                };
                if let Some(mut result) = conventional {
                    for part in parts {
                        if let Component::Normal(value) = part {
                            result.push(value);
                        }
                    }
                    return result;
                }
            }
        }
        path
    };
    let mut children = Children(vec![
        std::process::Command::new("node")
            .arg(node_path(
                f.0.join("node_modules/@anthropic-ai/claude-code/cli.js"),
            ))
            .spawn()
            .unwrap(),
        std::process::Command::new("node")
            .arg(node_path(f.0.join("other.js")))
            .spawn()
            .unwrap(),
    ]);
    let pid = children.0[0].id() as i32;
    let other = children.0[1].id() as i32;
    let mut target = None;
    for _ in 0..20 {
        assert!(children.0[0].try_wait().unwrap().is_none(),
            "Node CLI fixture exited before process detection; check external path compatibility and Node stderr");
        assert!(
            children.0[1].try_wait().unwrap().is_none(),
            "Unrelated Node fixture exited before process detection; check Node stderr"
        );
        let current = processes().unwrap();
        assert!(!current.iter().any(|p| p.pid == other));
        target = current.into_iter().find(|p| p.pid == pid);
        if target.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let target = target.expect("必须识别 Node 启动的官方 CLI 入口");
    assert_eq!(target.name, "Claude Code");
    signal_same_process(&target, 9).unwrap();
    assert!(!children.0[0].wait().unwrap().success());
    assert!(children.0[1].try_wait().unwrap().is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn linux_cleanup_uses_xdg_cache_paths_and_preserves_desktop_configuration() {
    let f = Fixture::new();
    f.file(".cache/claude-cli-nodejs/data", "cache");
    f.file(".config/Claude/Cache/data", "cache");
    f.file(".config/Claude/claude_desktop_config.json", "settings");
    f.file(
        "Library/Caches/claude-cli-nodejs/data",
        "not a Linux target",
    );
    let plan = collect(&f.0, None).unwrap();
    assert_eq!(plan.items.len(), 2);
    let state = Cleaner::default();
    let result = perform_items(
        &f.0,
        &plan.items,
        &state,
        || Ok(false),
        &Reporter::channel(None),
    )
    .unwrap();
    assert!(result.completed);
    assert!(f
        .0
        .join(".config/Claude/claude_desktop_config.json")
        .exists());
    assert!(f.0.join("Library/Caches/claude-cli-nodejs/data").exists());
}
