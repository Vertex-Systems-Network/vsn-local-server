//! VSN Devify framework catalog (DVF-02): one-click project creation for CMSs and
//! frameworks beyond the frozen PKG-02 built-in templates. Every generator runs
//! non-interactively in the destination's parent folder with the destination path as
//! an argument, so nothing outside the chosen folder is touched.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const CREATE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const OUTPUT_TAIL_BYTES: usize = 8 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    #[error("invalid framework request: {0}")]
    Invalid(String),
    #[error("{0} is not installed or not on PATH")]
    MissingTool(String),
    #[error("framework io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Framework {
    pub id: String,
    pub name: String,
    pub language: String,
    /// Tools the generator needs on PATH (or in the space).
    pub requires: Vec<String>,
}

struct Entry {
    id: &'static str,
    name: &'static str,
    language: &'static str,
    requires: &'static [&'static str],
    program: &'static str,
    /// `{dir}` is the destination folder name (generators run in its parent folder);
    /// `{dest}` is the absolute destination path.
    args: &'static [&'static str],
}

const CATALOG: &[Entry] = &[
    Entry {
        id: "wordpress",
        name: "WordPress",
        language: "PHP",
        requires: &["php", "wp"],
        program: "wp",
        args: &["core", "download", "--path={dest}"],
    },
    Entry {
        id: "drupal",
        name: "Drupal",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "drupal/recommended-project",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "symfony",
        name: "Symfony",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "symfony/skeleton",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "codeigniter",
        name: "CodeIgniter 4",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "codeigniter4/appstarter",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "cakephp",
        name: "CakePHP",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "--prefer-dist",
            "cakephp/app",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "yii",
        name: "Yii 2",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "yiisoft/yii2-app-basic",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "slim",
        name: "Slim",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "slim/slim-skeleton",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "opencart",
        name: "OpenCart",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "opencart/opencart",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "magento",
        name: "Magento Open Source (Mage-OS mirror)",
        language: "PHP",
        requires: &["php", "composer"],
        program: "composer",
        args: &[
            "create-project",
            "--repository-url=https://mirror.mage-os.org/",
            "magento/project-community-edition",
            "{dir}",
            "--no-interaction",
        ],
    },
    Entry {
        id: "nextjs",
        name: "Next.js",
        language: "JavaScript",
        requires: &["node", "npx"],
        program: "npx",
        args: &["--yes", "create-next-app@latest", "{dir}", "--yes"],
    },
    Entry {
        id: "vite-react",
        name: "React (Vite)",
        language: "JavaScript",
        requires: &["node", "npm"],
        program: "npm",
        args: &[
            "create",
            "--yes",
            "vite@latest",
            "{dir}",
            "--",
            "--template",
            "react-ts",
        ],
    },
    Entry {
        id: "vite-vue",
        name: "Vue (Vite)",
        language: "JavaScript",
        requires: &["node", "npm"],
        program: "npm",
        args: &[
            "create",
            "--yes",
            "vite@latest",
            "{dir}",
            "--",
            "--template",
            "vue-ts",
        ],
    },
    Entry {
        id: "astro",
        name: "Astro",
        language: "JavaScript",
        requires: &["node", "npm"],
        program: "npm",
        args: &[
            "create",
            "--yes",
            "astro@latest",
            "{dir}",
            "--",
            "--template",
            "minimal",
            "--yes",
            "--no-git",
            "--skip-houston",
        ],
    },
    Entry {
        id: "nuxt",
        name: "Nuxt",
        language: "JavaScript",
        requires: &["node", "npx"],
        program: "npx",
        args: &[
            "--yes",
            "nuxi@latest",
            "init",
            "{dir}",
            "--packageManager",
            "npm",
            "--gitInit",
            "false",
        ],
    },
    Entry {
        id: "express",
        name: "Express",
        language: "JavaScript",
        requires: &["node", "npx"],
        program: "npx",
        args: &["--yes", "express-generator", "--view=ejs", "{dir}"],
    },
    Entry {
        id: "nestjs",
        name: "NestJS",
        language: "TypeScript",
        requires: &["node", "npx"],
        program: "npx",
        args: &[
            "--yes",
            "@nestjs/cli@latest",
            "new",
            "{dir}",
            "--package-manager",
            "npm",
            "--skip-git",
        ],
    },
    Entry {
        id: "rails",
        name: "Ruby on Rails",
        language: "Ruby",
        requires: &["ruby", "rails"],
        program: "rails",
        args: &["new", "{dir}", "--skip-git"],
    },
    Entry {
        id: "shopify-theme",
        name: "Shopify theme",
        language: "Liquid",
        requires: &["shopify", "git"],
        program: "shopify",
        args: &["theme", "init", "{dir}"],
    },
];

pub fn catalog() -> Vec<Framework> {
    CATALOG
        .iter()
        .map(|e| Framework {
            id: e.id.into(),
            name: e.name.into(),
            language: e.language.into(),
            requires: e.requires.iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrameworkPlan {
    pub framework: String,
    pub destination: PathBuf,
    pub program: String,
    pub args: Vec<String>,
    pub requires: Vec<String>,
}

/// What would run, without running it. The destination must be absolute, its parent must
/// exist and the destination itself must be absent or an empty folder.
pub fn plan(id: &str, destination: &Path) -> Result<FrameworkPlan, FrameworkError> {
    let entry = CATALOG
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| FrameworkError::Invalid(format!("unknown framework {id}")))?;
    if !destination.is_absolute() {
        return Err(FrameworkError::Invalid(
            "destination must be an absolute path".into(),
        ));
    }
    let dest = destination.to_string_lossy();
    let dir = destination
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if dir.is_empty()
        || dir.starts_with(['-', '.'])
        || dest.contains(['\n', '\r', '\0'])
        || !dir
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(FrameworkError::Invalid(
            "folder name must use letters, digits, '-', '_' or '.' and not start with '-' or '.'"
                .into(),
        ));
    }
    Ok(FrameworkPlan {
        framework: entry.id.into(),
        destination: destination.to_path_buf(),
        program: entry.program.into(),
        args: entry
            .args
            .iter()
            .map(|a| a.replace("{dest}", &dest).replace("{dir}", &dir))
            .collect(),
        requires: entry.requires.iter().map(|s| s.to_string()).collect(),
    })
}

/// Find `program` on PATH; on Windows prefer real executables and shims (`.exe`, `.cmd`,
/// `.bat`) over npm's extension-less shell scripts.
pub fn resolve_program(program: &str, path_var: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    let dirs: Vec<PathBuf> = path_var
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    for dir in &dirs {
        for ext in exts {
            let candidate = dir.join(format!("{program}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrameworkResult {
    pub framework: String,
    pub destination: PathBuf,
    pub status_code: Option<i32>,
    pub timed_out: bool,
    /// Last bytes of combined stdout/stderr.
    pub output_tail: String,
}

fn destination_ready(destination: &Path) -> Result<(), FrameworkError> {
    let parent = destination
        .parent()
        .ok_or_else(|| FrameworkError::Invalid("destination needs a parent folder".into()))?;
    if !parent.is_dir() {
        return Err(FrameworkError::Invalid(
            "the parent folder must already exist".into(),
        ));
    }
    if destination.exists() {
        if !destination.is_dir() || fs::read_dir(destination)?.next().is_some() {
            return Err(FrameworkError::Invalid(
                "destination must be a new or empty folder".into(),
            ));
        }
        // Generators such as create-next-app refuse an existing folder; remove the empty one.
        fs::remove_dir(destination)?;
    }
    Ok(())
}

fn tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(OUTPUT_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

/// Run the generator. `env` is added to the inherited environment (a space's PATH and caches).
pub fn create(
    plan: &FrameworkPlan,
    env: &[(String, String)],
) -> Result<FrameworkResult, FrameworkError> {
    destination_ready(&plan.destination)?;
    let parent = plan.destination.parent().unwrap_or(Path::new("."));
    run_tool(
        &plan.framework,
        &plan.destination,
        &plan.program,
        &plan.args,
        parent,
        env,
    )
}

pub fn run_tool(
    label: &str,
    destination: &Path,
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &[(String, String)],
) -> Result<FrameworkResult, FrameworkError> {
    let path_var = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| std::ffi::OsString::from(v))
        .or_else(|| std::env::var_os("PATH"));
    let resolved = resolve_program(program, path_var.as_deref())
        .ok_or_else(|| FrameworkError::MissingTool(program.to_string()))?;
    let mut command = Command::new(resolved);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        command.env(k, v);
    }
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: composer, npm and the dump tools never flash a console.
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let mut out = child.stdout.take().expect("piped stdout");
    let mut err = child.stderr.take().expect("piped stderr");
    let out_thread = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err_thread = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let started = Instant::now();
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (Some(status), false);
        }
        if started.elapsed() > CREATE_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    let mut output = out_thread.join().unwrap_or_default();
    output.extend(err_thread.join().unwrap_or_default());
    Ok(FrameworkResult {
        framework: label.to_string(),
        destination: destination.to_path_buf(),
        status_code: status.and_then(|s| s.code()),
        timed_out,
        output_tail: tail(&output),
    })
}

/// One update step for an existing project, chosen from the files it contains.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateStep {
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
}

/// What "Update" would run in `project`: WordPress core and plugins, Composer, the
/// JavaScript and Python package managers the lockfiles name, Bundler, Go modules
/// and Cargo. Nothing is guessed.
pub fn update_plan(project: &Path) -> Vec<UpdateStep> {
    let has = |f: &str| project.join(f).is_file();
    let step = |label: &str, program: &str, args: &[&str]| UpdateStep {
        label: label.into(),
        program: program.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
    };
    let mut steps = Vec::new();
    if has("wp-config.php") || has("wp-config-sample.php") {
        steps.push(step("WordPress core", "wp", &["core", "update"]));
        steps.push(step(
            "WordPress plugins",
            "wp",
            &["plugin", "update", "--all"],
        ));
    }
    if has("composer.json") {
        steps.push(step(
            "Composer packages",
            "composer",
            &["update", "--no-interaction"],
        ));
    }
    if has("package.json") {
        if has("bun.lock") || has("bun.lockb") {
            steps.push(step("Bun packages", "bun", &["update"]));
        } else if has("pnpm-lock.yaml") {
            steps.push(step("pnpm packages", "pnpm", &["update"]));
        } else if has("yarn.lock") {
            steps.push(step("Yarn packages", "yarn", &["upgrade"]));
        } else {
            steps.push(step("npm packages", "npm", &["update"]));
        }
    }
    if has("Gemfile") {
        steps.push(step("Ruby gems", "bundle", &["update"]));
    }
    if has("poetry.lock") {
        steps.push(step("Poetry packages", "poetry", &["update"]));
    } else if has("uv.lock") {
        steps.push(step("uv packages", "uv", &["sync", "--upgrade"]));
    } else if has("Pipfile") {
        steps.push(step("Pipenv packages", "pipenv", &["update"]));
    } else if has("requirements.txt") {
        steps.push(step(
            "pip packages",
            "python",
            &[
                "-m",
                "pip",
                "install",
                "--upgrade",
                "-r",
                "requirements.txt",
            ],
        ));
    }
    if has("go.mod") {
        steps.push(step("Go modules", "go", &["get", "-u", "./..."]));
        steps.push(step("Go module tidy", "go", &["mod", "tidy"]));
    }
    if has("Cargo.toml") {
        steps.push(step("Cargo crates", "cargo", &["update"]));
    }
    steps
}

/// Run every update step in order; stops at the first failure.
pub fn update(
    project: &Path,
    env: &[(String, String)],
) -> Result<Vec<FrameworkResult>, FrameworkError> {
    if !project.is_dir() {
        return Err(FrameworkError::Invalid(
            "project folder does not exist".into(),
        ));
    }
    let mut results = Vec::new();
    for s in update_plan(project) {
        let r = run_tool(&s.label, project, &s.program, &s.args, project, env)?;
        let failed = r.timed_out || r.status_code != Some(0);
        results.push(r);
        if failed {
            break;
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_plan_follows_project_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(update_plan(dir.path()).is_empty());
        std::fs::write(dir.path().join("composer.json"), "{}").unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        let labels: Vec<String> = update_plan(dir.path())
            .into_iter()
            .map(|s| s.program)
            .collect();
        assert_eq!(labels, vec!["composer", "pnpm"]);
    }

    #[test]
    fn update_plan_covers_bun_python_go_and_cargo() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "package.json",
            "bun.lock",
            "pnpm-lock.yaml",
            "poetry.lock",
            "requirements.txt",
            "go.mod",
            "Cargo.toml",
        ] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        let programs: Vec<String> = update_plan(dir.path())
            .into_iter()
            .map(|s| s.program)
            .collect();
        assert_eq!(programs, vec!["bun", "poetry", "go", "go", "cargo"]);

        let py = tempfile::tempdir().unwrap();
        std::fs::write(py.path().join("requirements.txt"), "").unwrap();
        let plan = update_plan(py.path());
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].program, "python");
        assert_eq!(plan[0].args.last().unwrap(), "requirements.txt");
    }

    #[test]
    fn catalog_ids_are_unique_and_plans_substitute_destination() {
        let ids: Vec<String> = catalog().into_iter().map(|f| f.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
        for id in [
            "wordpress",
            "symfony",
            "codeigniter",
            "nextjs",
            "shopify-theme",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id} missing");
        }
        let dest = std::env::temp_dir().join("shop");
        let p = plan("symfony", &dest).unwrap();
        assert_eq!(p.program, "composer");
        assert!(p.args.contains(&"shop".to_string()));
        let wp = plan("wordpress", &dest).unwrap();
        assert_eq!(
            wp.args.last().unwrap(),
            &format!("--path={}", dest.to_string_lossy())
        );
    }

    #[test]
    fn rejects_unknown_relative_and_non_empty() {
        assert!(plan("joomla-9000", &std::env::temp_dir().join("x")).is_err());
        assert!(plan("symfony", Path::new("relative/dir")).is_err());
        assert!(plan("symfony", &std::env::temp_dir().join("-rf")).is_err());
        assert!(plan("symfony", &std::env::temp_dir().join("my app")).is_err());
        let dir = tempfile::tempdir().unwrap();
        let busy = dir.path().join("busy");
        fs::create_dir(&busy).unwrap();
        fs::write(busy.join("keep.txt"), b"x").unwrap();
        assert!(destination_ready(&busy).is_err());
        let empty = dir.path().join("empty");
        fs::create_dir(&empty).unwrap();
        assert!(destination_ready(&empty).is_ok() && !empty.exists());
    }

    #[test]
    fn resolves_programs_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "fakegen.cmd"
        } else {
            "fakegen"
        };
        fs::write(dir.path().join(name), b"").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(
            resolve_program("fakegen", Some(&path)),
            Some(dir.path().join(name))
        );
        assert_eq!(resolve_program("nothere", Some(&path)), None);
    }

    #[cfg(unix)]
    #[test]
    fn create_runs_generator_and_reports_output() {
        use std::os::unix::fs::PermissionsExt;
        let bin = tempfile::tempdir().unwrap();
        let script = bin.path().join("composer");
        fs::write(
            &script,
            "#!/bin/sh\nmkdir -p \"$3\" && echo created \"$2\"\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let work = tempfile::tempdir().unwrap();
        let dest = work.path().join("app");
        let p = plan("symfony", &dest).unwrap();
        let mut dirs = vec![bin.path().to_path_buf()];
        dirs.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let path = std::env::join_paths(dirs)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let r = create(&p, &[("PATH".into(), path)]).unwrap();
        assert_eq!(r.status_code, Some(0));
        assert!(r.output_tail.contains("created symfony/skeleton"));
        assert!(dest.is_dir());
    }
}
