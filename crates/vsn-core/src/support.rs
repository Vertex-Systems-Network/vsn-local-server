//! One-click "Diagnose" for the Help screen: a plain checklist of what works, what is
//! missing and how to fix it. Read-only; it never changes anything.

use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DiagnoseCheck {
    pub area: String,
    pub check: String,
    pub ok: bool,
    pub detail: String,
    /// What to do when the check fails (empty when it passes).
    pub fix: String,
}

fn check(area: &str, name: &str, ok: bool, detail: String, fix: &str) -> DiagnoseCheck {
    DiagnoseCheck {
        area: area.into(),
        check: name.into(),
        ok,
        detail,
        fix: if ok { String::new() } else { fix.into() },
    }
}

/// Optional tools and what each one unlocks; missing ones are reported, not errors.
const TOOLS: &[(&str, &str, &str)] = &[
    (
        "git",
        "Source control (Git)",
        "Install Git for Windows: winget install Git.Git",
    ),
    (
        "svn",
        "Source control (SVN)",
        "Install TortoiseSVN with command-line tools, or winget install Slik.Subversion",
    ),
    (
        "php",
        "PHP projects",
        "Pin PHP in the space or install it: winget install PHP.PHP.8.3",
    ),
    (
        "composer",
        "Laravel, Symfony, Drupal, Magento and other Composer frameworks",
        "winget install Composer.Composer",
    ),
    (
        "node",
        "Node.js projects",
        "Pin Node in the space or winget install OpenJS.NodeJS.LTS",
    ),
    (
        "caddy",
        "name.test domains routed to your app",
        "winget install CaddyServer.Caddy",
    ),
    ("mailpit", "Mail catcher", "winget install axllent.mailpit"),
    (
        "redis-server",
        "Redis service",
        "Install Redis or Memurai on Windows",
    ),
    (
        "rclone",
        "Cloud backups (Drive, OneDrive, Dropbox, S3, MEGA …)",
        "winget install Rclone.Rclone",
    ),
    (
        "curl",
        "Email backups and API tools",
        "curl ships with Windows 10 and later",
    ),
    (
        "tar",
        "Backup archives",
        "tar ships with Windows 10 and later",
    ),
    (
        "cloudflared",
        "Public internet links (Cloudflare)",
        "winget install Cloudflare.cloudflared",
    ),
    (
        "ngrok",
        "Public internet links (ngrok)",
        "winget install ngrok.ngrok",
    ),
    (
        "frpc",
        "Public internet links (your frp server)",
        "Download frp from github.com/fatedier/frp",
    ),
];

pub fn support_diagnose(principal: &Principal) -> Result<Vec<DiagnoseCheck>, CoreError> {
    vsn_policy::require(principal, Permission::MachineView)?;
    let mut out = Vec::new();
    out.push(check(
        "Agent",
        "VSN Agent is running",
        true,
        format!("version {}", env!("CARGO_PKG_VERSION")),
        "",
    ));

    match vsn_security::data_dir() {
        Ok(dir) => {
            let probe = dir.join(".diagnose-write-test");
            let writable = std::fs::write(&probe, b"ok").is_ok();
            let _ = std::fs::remove_file(&probe);
            out.push(check(
                "Agent",
                "Data folder is writable",
                writable,
                dir.display().to_string(),
                "Check disk space and that antivirus is not blocking the VSN data folder",
            ));
        }
        Err(e) => out.push(check(
            "Agent",
            "Data folder",
            false,
            e.to_string(),
            "Reinstall VSN Devify",
        )),
    }

    match vsn_audit::default_audit_path().map(|p| (vsn_audit::verify(&p), p)) {
        Ok((Ok(count), _)) => out.push(check("Security", "Audit log chain is intact", true, format!("{count} events"), "")),
        Ok((Err(e), path)) => out.push(check(
            "Security",
            "Audit log chain is intact",
            false,
            format!("{e} ({})", path.display()),
            "The audit log was changed outside VSN. Keep a copy for investigation, then contact support",
        )),
        Err(e) => out.push(check("Security", "Audit log", false, e.to_string(), "Reinstall VSN Devify")),
    }

    let roots = workspace_roots(principal).unwrap_or_default();
    out.push(check(
        "Workspace",
        "At least one workspace folder is set",
        !roots.is_empty(),
        format!("{} folder(s)", roots.len()),
        "Open Settings and add the folder that holds your projects",
    ));
    for root in &roots {
        out.push(check(
            "Workspace",
            "Workspace folder exists",
            root.is_dir(),
            root.display().to_string(),
            "Create the folder again or remove it in Settings",
        ));
    }

    let locks = load_space_locks().unwrap_or_default();
    for root in load_space_registry().unwrap_or_default() {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if locks.contains_key(&root) && !space_is_unlocked(&root) {
            out.push(check(
                "Spaces",
                &format!("{name}: locked"),
                true,
                "unlock it to inspect services".into(),
                "",
            ));
            continue;
        }
        match vsn_space::load(&root) {
            Ok(spec) => {
                for service in &spec.services {
                    let Some(port) = service.port else { continue };
                    let up = std::net::TcpStream::connect_timeout(
                        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                        std::time::Duration::from_millis(200),
                    )
                    .is_ok();
                    out.push(check(
                        "Spaces",
                        &format!("{}: {} on port {port}", spec.name, service.kind),
                        up,
                        if up { "answering".into() } else { "not answering".into() },
                        "Start it in Spaces; if it still fails, another program may use the port — change it with Change port",
                    ));
                }
            }
            Err(e) => out.push(check(
                "Spaces",
                &format!("{name}: space.toml"),
                false,
                e.to_string(),
                "Open the space in Spaces and save it again",
            )),
        }
    }

    for (tool, unlocks, fix) in TOOLS {
        let found = vsn_system::find_executable(tool);
        out.push(check(
            "Tools",
            &format!("{tool} — {unlocks}"),
            found.is_ok(),
            found
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "not found on PATH".into()),
            fix,
        ));
    }
    Ok(out)
}
