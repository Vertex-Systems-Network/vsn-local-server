//! DVF-10 profiler hooks: a space process can be started with profiling on. Node writes
//! V8 `.cpuprofile` files (via NODE_OPTIONS, so npm/yarn scripts and child processes are
//! covered) and PHP with Xdebug writes cachegrind files, all into `.devify/profiles`.
//! Node profiles are summarised as the functions with the most self time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::CoreError;

pub const PROFILE_DIR: &str = ".devify/profiles";

/// Environment that turns on Node and PHP (Xdebug) profiling into `dir`.
pub fn profile_env(dir: &Path, existing_node_options: Option<&str>) -> Vec<(String, String)> {
    let d = dir.to_string_lossy();
    let node = format!("--cpu-prof --cpu-prof-dir=\"{d}\"");
    let node = match existing_node_options.filter(|s| !s.trim().is_empty()) {
        Some(prev) => format!("{prev} {node}"),
        None => node,
    };
    vec![
        ("NODE_OPTIONS".into(), node),
        ("XDEBUG_MODE".into(), "profile".into()),
        ("XDEBUG_CONFIG".into(), format!("output_dir={d}")),
    ]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HotFunction {
    pub function: String,
    pub location: String,
    pub self_ms: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ProfileFile {
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_unix: u64,
    /// "node" (.cpuprofile) or "php" (cachegrind.out.*).
    pub kind: String,
    pub total_ms: Option<f64>,
    pub hot: Vec<HotFunction>,
}

#[derive(serde::Deserialize)]
struct CallFrame {
    #[serde(rename = "functionName", default)]
    function_name: String,
    #[serde(default)]
    url: String,
    #[serde(rename = "lineNumber", default)]
    line_number: i64,
}
#[derive(serde::Deserialize)]
struct Node {
    id: u64,
    #[serde(rename = "callFrame")]
    call_frame: CallFrame,
}
#[derive(serde::Deserialize)]
struct CpuProfile {
    nodes: Vec<Node>,
    #[serde(default)]
    samples: Vec<u64>,
    #[serde(rename = "timeDeltas", default)]
    time_deltas: Vec<i64>,
}

/// Self time per function from a V8 `.cpuprofile`, largest first. Idle, GC and
/// program pseudo-frames are left out of the list but counted in the total.
pub fn summarize_cpuprofile(json: &str, top: usize) -> Result<(f64, Vec<HotFunction>), CoreError> {
    let p: CpuProfile = serde_json::from_str(json)
        .map_err(|e| CoreError::Rejected(format!("not a V8 cpuprofile: {e}")))?;
    let frames: HashMap<u64, &CallFrame> = p.nodes.iter().map(|n| (n.id, &n.call_frame)).collect();
    let mut by_fn: HashMap<(String, String), f64> = HashMap::new();
    let mut total_us = 0f64;
    for (i, sample) in p.samples.iter().enumerate() {
        let delta = p.time_deltas.get(i).copied().unwrap_or(0).max(0) as f64;
        total_us += delta;
        let Some(frame) = frames.get(sample) else {
            continue;
        };
        let name = if frame.function_name.is_empty() {
            "(anonymous)".to_string()
        } else {
            frame.function_name.clone()
        };
        if matches!(
            name.as_str(),
            "(idle)" | "(program)" | "(garbage collector)" | "(root)"
        ) {
            continue;
        }
        let location = if frame.url.is_empty() {
            String::new()
        } else {
            format!("{}:{}", frame.url, frame.line_number + 1)
        };
        *by_fn.entry((name, location)).or_default() += delta;
    }
    let mut hot: Vec<HotFunction> = by_fn
        .into_iter()
        .map(|((function, location), us)| HotFunction {
            function,
            location,
            self_ms: us / 1000.0,
            percent: if total_us > 0.0 {
                us * 100.0 / total_us
            } else {
                0.0
            },
        })
        .collect();
    hot.sort_by(|a, b| {
        b.self_ms
            .total_cmp(&a.self_ms)
            .then(a.function.cmp(&b.function))
    });
    hot.truncate(top);
    Ok((total_us / 1000.0, hot))
}

/// Newest first; Node profiles above 64 MiB are listed without a summary.
pub fn list_profiles(space_root: &Path) -> Result<Vec<ProfileFile>, CoreError> {
    let dir = space_root.join(PROFILE_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = if name.ends_with(".cpuprofile") {
            "node"
        } else if name.starts_with("cachegrind.out") {
            "php"
        } else {
            continue;
        };
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let modified_unix = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (total_ms, hot) = if kind == "node" && meta.len() <= 64 * 1024 * 1024 {
            match std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| summarize_cpuprofile(&s, 15).ok())
            {
                Some((t, h)) => (Some(t), h),
                None => (None, Vec::new()),
            }
        } else {
            (None, Vec::new())
        };
        out.push(ProfileFile {
            name,
            path,
            bytes: meta.len(),
            modified_unix,
            kind: kind.into(),
            total_ms,
            hot,
        });
    }
    out.sort_by(|a, b| {
        b.modified_unix
            .cmp(&a.modified_unix)
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r#"{"nodes":[
        {"id":1,"callFrame":{"functionName":"(root)","url":"","lineNumber":-1}},
        {"id":2,"callFrame":{"functionName":"slowLoop","url":"file:///app/a.js","lineNumber":9}},
        {"id":3,"callFrame":{"functionName":"","url":"file:///app/b.js","lineNumber":0}},
        {"id":4,"callFrame":{"functionName":"(idle)","url":"","lineNumber":-1}}],
        "startTime":0,"endTime":10000,
        "samples":[2,2,3,4,2],"timeDeltas":[1000,3000,1000,4000,1000]}"#;

    #[test]
    fn self_time_is_summed_per_function() {
        let (total, hot) = summarize_cpuprofile(PROFILE, 10).unwrap();
        assert_eq!(total, 10.0);
        assert_eq!(hot[0].function, "slowLoop");
        assert_eq!(hot[0].location, "file:///app/a.js:10");
        assert_eq!(hot[0].self_ms, 5.0);
        assert_eq!(hot[0].percent, 50.0);
        assert_eq!(hot[1].function, "(anonymous)");
        assert_eq!(hot.len(), 2, "idle is left out");
        assert!(summarize_cpuprofile("{}", 5).is_err());
    }

    #[test]
    fn env_appends_to_existing_node_options() {
        let env = profile_env(
            Path::new("/s/.devify/profiles"),
            Some("--max-old-space-size=4096"),
        );
        assert_eq!(
            env[0].1,
            "--max-old-space-size=4096 --cpu-prof --cpu-prof-dir=\"/s/.devify/profiles\""
        );
        assert_eq!(env[1], ("XDEBUG_MODE".into(), "profile".into()));
    }

    #[test]
    fn lists_newest_profiles_with_summary() {
        let root = std::env::temp_dir().join(format!("vsn-profile-test-{}", std::process::id()));
        let p = root.join(PROFILE_DIR);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("CPU.1.cpuprofile"), PROFILE).unwrap();
        std::fs::write(p.join("cachegrind.out.42"), "events: Time").unwrap();
        std::fs::write(p.join("notes.txt"), "x").unwrap();
        let list = list_profiles(&root).unwrap();
        assert_eq!(list.len(), 2);
        let node = list.iter().find(|f| f.kind == "node").unwrap();
        assert_eq!(node.hot[0].function, "slowLoop");
        assert!(list_profiles(&root.join("missing")).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
