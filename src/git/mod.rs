use std::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::lore::{AtomState, LoreAtom, LoreKind, Workspace, WorkspaceState};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitTrailer {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitMessage {
    pub subject: String,
    pub body: Option<String>,
    pub trailers: Vec<CommitTrailer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoricalDecision {
    pub commit_hash: String,
    pub subject: String,
    pub trailer: CommitTrailer,
    pub file_path: PathBuf,
}

pub fn discover_repository(path: impl AsRef<Path>) -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path.as_ref())
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .with_context(|| format!("failed to execute git for {}", path.as_ref().display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "failed to discover git repository from {}: {}",
            path.as_ref().display(),
            stderr
        ));
    }

    let root = String::from_utf8(output.stdout)
        .with_context(|| format!("git returned invalid utf-8 for {}", path.as_ref().display()))?;
    let root = PathBuf::from(root.trim());
    Ok(fs::canonicalize(&root).unwrap_or(root))
}

pub fn repository_root(repository: &Path) -> PathBuf {
    repository.to_path_buf()
}

pub fn render_commit_trailers(atoms: &[LoreAtom]) -> String {
    atoms
        .iter()
        .map(|atom| format!("{}: [{}] {}", trailer_key(&atom.kind), atom.id, atom.title))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_commit_message(subject: impl AsRef<str>, atoms: &[LoreAtom]) -> String {
    let subject = subject.as_ref().trim();
    let trailers = render_commit_trailers(atoms);

    if trailers.is_empty() {
        subject.to_string()
    } else {
        format!("{subject}\n\n{trailers}")
    }
}

pub fn commit_lore_message(
    repository_root: impl AsRef<Path>,
    message: impl AsRef<str>,
    allow_empty: bool,
) -> Result<String> {
    let repository_root = repository_root.as_ref();
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repository_root)
        .arg("-c")
        .arg("user.name=Git-Lore")
        .arg("-c")
        .arg("user.email=git-lore@localhost")
        .arg("commit")
        .arg("--cleanup=verbatim")
        .arg("-F")
        .arg("-");

    if allow_empty {
        command.arg("--allow-empty");
    }

    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn git commit in {}", repository_root.display()))?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(message.as_ref().as_bytes())
            .with_context(|| format!("failed to write commit message in {}", repository_root.display()))?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to finish git commit in {}", repository_root.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "git commit failed in {}: {}",
            repository_root.display(),
            stderr
        ));
    }

    let hash_output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .with_context(|| format!("failed to read commit hash in {}", repository_root.display()))?;

    if !hash_output.status.success() {
        let stderr = String::from_utf8_lossy(&hash_output.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "git rev-parse failed in {}: {}",
            repository_root.display(),
            stderr
        ));
    }

    let hash = String::from_utf8(hash_output.stdout)
        .with_context(|| format!("git returned invalid utf-8 in {}", repository_root.display()))?;
    Ok(hash.trim().to_string())
}

pub fn write_lore_ref(repository_root: impl AsRef<Path>, atom: &LoreAtom, source_commit: &str) -> Result<()> {
    let repository_root = repository_root.as_ref();
    run_git(
        repository_root,
        &[
            "update-ref",
            &format!("refs/lore/accepted/{}", atom.id),
            source_commit,
        ],
    )?;

    let note = serde_json::to_string_pretty(atom)?;
    run_git(
        repository_root,
        &[
            "notes",
            "--ref=refs/notes/lore",
            "add",
            "-f",
            "-m",
            &note,
            source_commit,
        ],
    )?;

    Ok(())
}

pub fn list_lore_refs(repository_root: impl AsRef<Path>) -> Result<Vec<(String, String)>> {
    let output = run_git_output(
        repository_root.as_ref(),
        &[
            "for-each-ref",
            "refs/lore/accepted",
            "--format=%(refname) %(objectname)",
        ],
    )?;

    Ok(output
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(name, hash)| (name.to_string(), hash.to_string()))
        .collect())
}

pub fn collect_recent_decisions_for_path(
    repository_root: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
    limit: usize,
) -> Result<Vec<HistoricalDecision>> {
    let file_path = file_path.as_ref().to_path_buf();
    let file_path_arg = file_path.to_string_lossy().to_string();
    let output = run_git_output(
        repository_root.as_ref(),
        &[
            "log",
            "--follow",
            "--format=%H%x1f%B%x1e",
            "--",
            file_path_arg.as_str(),
        ],
    )?;

    let mut decisions = Vec::new();

    for record in output.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }

        let Some((commit_hash, message)) = record.split_once('\x1f') else {
            continue;
        };

        let parsed = parse_commit_message(message.trim());
        for trailer in parsed.trailers.into_iter().filter(|trailer| trailer.key == "Lore-Decision") {
            decisions.push(HistoricalDecision {
                commit_hash: commit_hash.trim().to_string(),
                subject: parsed.subject.clone(),
                trailer,
                file_path: file_path.clone(),
            });

            if decisions.len() >= limit {
                return Ok(decisions);
            }
        }
    }

    Ok(decisions)
}

pub fn install_git_lore_integration(repository_root: impl AsRef<Path>) -> Result<()> {
    let repository_root = repository_root.as_ref();
    let git_dir = git_dir(repository_root)?;
    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    run_git(
        repository_root,
        &["config", "merge.lore.name", "Git-Lore Reasoning Merger"],
    )?;
    run_git(
        repository_root,
        &["config", "merge.lore.driver", "git-lore merge %O %A %B"],
    )?;

    write_hook(
        &hooks_dir.join("pre-commit"),
        "#!/bin/sh\nset -eu\nROOT=\"$(git rev-parse --show-toplevel)\"\nif [ -x \"$ROOT/git-lore\" ]; then\n  \"$ROOT/git-lore\" validate .\nelse\n  git-lore validate .\nfi\n",
    )?;
    write_hook(
        &hooks_dir.join("post-checkout"),
        "#!/bin/sh\nset -eu\nROOT=\"$(git rev-parse --show-toplevel)\"\nif [ -x \"$ROOT/git-lore\" ]; then\n  \"$ROOT/git-lore\" sync .\nelse\n  git-lore sync .\nfi\n",
    )?;

    Ok(())
}

pub fn validate_workspace_against_git(repository_root: impl AsRef<Path>, workspace: &Workspace) -> Result<Vec<String>> {
    let repository_root = repository_root.as_ref();
    let mut issues = Vec::new();

    for issue in workspace.sanitize_report()? {
        issues.push(format!(
            "sensitive content in {}.{}: {}",
            issue.atom_id, issue.field, issue.reason
        ));
    }

    let state = workspace.load_state()?.atoms;
    for violation in workspace.scan_prism_hard_locks(&state)? {
        issues.push(format!("{} ({})", violation.message, violation.atom_ids.join(", ")));
    }

    for issue in workspace.validation_report()? {
        issues.push(format!(
            "validation failed for {}: {}",
            issue.atom_id, issue.reason
        ));
    }

    for (refname, objectname) in list_lore_refs(repository_root)? {
        if refname.is_empty() || objectname.is_empty() {
            issues.push("empty lore ref entry detected".to_string());
        }
    }

    Ok(issues)
}

pub fn sync_workspace_from_git_history(
    repository_root: impl AsRef<Path>,
    workspace: &Workspace,
) -> Result<Vec<LoreAtom>> {
    let repository_root = repository_root.as_ref();
    let state = workspace.load_state()?;
    let mut atoms_by_id = BTreeMap::<String, LoreAtom>::new();

    for atom in state.atoms {
        upsert_atom(&mut atoms_by_id, atom);
    }

    for (refname, objectname) in list_lore_refs(repository_root)? {
        if let Some(atom_id) = refname.rsplit('/').next() {
            let candidate = LoreAtom {
                id: atom_id.to_string(),
                kind: LoreKind::Decision,
                state: AtomState::Accepted,
                title: format!("Synced accepted lore from {objectname}"),
                body: Some(format!("Restored from {refname}")),
                scope: None,
                path: None,
                validation_script: None,
                created_unix_seconds: 0,
            };

            if let Some(existing) = atoms_by_id.get_mut(atom_id) {
                if should_replace_with_candidate(existing, &candidate) {
                    *existing = candidate;
                }
            } else {
                atoms_by_id.insert(atom_id.to_string(), candidate);
            }
        }
    }

    let atoms = atoms_by_id.into_values().collect::<Vec<_>>();

    workspace.set_state(&WorkspaceState {
        version: state.version,
        atoms: atoms.clone(),
    })?;
    Ok(atoms)
}

fn upsert_atom(atoms_by_id: &mut BTreeMap<String, LoreAtom>, atom: LoreAtom) {
    match atoms_by_id.get(&atom.id) {
        Some(existing) if !should_replace_with_candidate(existing, &atom) => {}
        _ => {
            atoms_by_id.insert(atom.id.clone(), atom);
        }
    }
}

fn should_replace_with_candidate(existing: &LoreAtom, candidate: &LoreAtom) -> bool {
    if candidate.created_unix_seconds > existing.created_unix_seconds {
        return true;
    }

    if candidate.created_unix_seconds < existing.created_unix_seconds {
        return false;
    }

    atom_preference_score(candidate) > atom_preference_score(existing)
}

fn atom_preference_score(atom: &LoreAtom) -> u8 {
    let mut score = 0u8;
    if atom.path.is_some() {
        score += 3;
    }
    if atom.scope.is_some() {
        score += 2;
    }
    if atom.body.is_some() {
        score += 2;
    }
    if atom.validation_script.is_some() {
        score += 1;
    }
    if !is_synced_placeholder(atom) {
        score += 1;
    }
    score
}

fn is_synced_placeholder(atom: &LoreAtom) -> bool {
    atom.created_unix_seconds == 0
        && atom.path.is_none()
        && atom.scope.is_none()
        && atom.title.starts_with("Synced accepted lore from ")
}

fn git_dir(repository_root: &Path) -> Result<PathBuf> {
    let output = run_git_output(repository_root, &["rev-parse", "--git-dir"])?;
    let git_dir = PathBuf::from(output.trim());
    if git_dir.is_absolute() {
        Ok(git_dir)
    } else {
        Ok(repository_root.join(git_dir))
    }
}

fn write_hook(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).with_context(|| format!("failed to write hook {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

fn run_git(repository_root: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute git in {}", repository_root.display()))?;

    if !status.success() {
        return Err(anyhow::anyhow!("git command failed in {}: {:?}", repository_root.display(), args));
    }

    Ok(())
}

fn run_git_output(repository_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git in {}", repository_root.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!("git command failed in {}: {}", repository_root.display(), stderr));
    }

    Ok(String::from_utf8(output.stdout).with_context(|| format!("git returned invalid utf-8 in {}", repository_root.display()))?)
}

pub fn parse_commit_message(message: &str) -> CommitMessage {
    let mut lines = message.lines().collect::<Vec<_>>();
    let trailer_start = lines
        .iter()
        .rposition(|line| line.trim().is_empty())
        .map(|index| index + 1)
        .unwrap_or(lines.len());
    let trailer_lines = if trailer_start < lines.len() {
        lines.split_off(trailer_start)
    } else {
        Vec::new()
    };

    let subject = lines.first().copied().unwrap_or_default().to_string();
    let body = if lines.len() > 2 {
        Some(lines[2..].join("\n"))
    } else {
        None
    };

    let trailers = trailer_lines
        .into_iter()
        .filter_map(|line| {
            let (key, value) = line.split_once(": ")?;
            Some(CommitTrailer {
                key: key.to_string(),
                value: value.to_string(),
            })
        })
        .collect();

    CommitMessage {
        subject,
        body,
        trailers,
    }
}

fn trailer_key(kind: &LoreKind) -> &'static str {
    match kind {
        LoreKind::Decision => "Lore-Decision",
        LoreKind::Assumption => "Lore-Assumption",
        LoreKind::OpenQuestion => "Lore-Open-Question",
        LoreKind::Signal => "Lore-Signal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn commit_message_round_trips_trailers() {
        let atom = LoreAtom {
            id: "ID-1".to_string(),
            kind: LoreKind::Decision,
            state: crate::lore::AtomState::Proposed,
            title: "Use Postgres".to_string(),
            body: None,
            scope: None,
            path: None,
            validation_script: None,
            created_unix_seconds: 0,
        };

        let message = build_commit_message("feat: add db layer", &[atom]);
        let parsed = parse_commit_message(&message);

        assert_eq!(parsed.subject, "feat: add db layer");
        assert_eq!(parsed.trailers.len(), 1);
        assert_eq!(parsed.trailers[0].key, "Lore-Decision");
        assert_eq!(parsed.trailers[0].value, "[ID-1] Use Postgres");
    }

    #[test]
    fn discovers_repository_root_from_nested_directory() {
        let root = std::env::temp_dir().join(format!("git-lore-git-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let status = Command::new("git").arg("-C").arg(&root).arg("init").status().unwrap();
        assert!(status.success());

        let nested = root.join("nested").join("folder");
        fs::create_dir_all(&nested).unwrap();

        let discovered_root = discover_repository(&nested).unwrap();
        let expected_root = fs::canonicalize(&root).unwrap_or(root);

        assert_eq!(discovered_root, expected_root);
    }

    #[test]
    fn commit_lore_message_creates_commit() {
        let root = std::env::temp_dir().join(format!("git-lore-commit-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let status = Command::new("git").arg("-C").arg(&root).arg("init").status().unwrap();
        assert!(status.success());

        let file_path = root.join("README.md");
        fs::write(&file_path, "hello\n").unwrap();
        let add_status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("add")
            .arg("README.md")
            .status()
            .unwrap();
        assert!(add_status.success());

        let hash = commit_lore_message(&root, "feat: add readme", true).unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn sync_workspace_is_idempotent_across_repeated_runs() {
        let root = std::env::temp_dir().join(format!("git-lore-sync-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();

        let init_status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .status()
            .unwrap();
        assert!(init_status.success());

        let workspace = Workspace::init(&root).unwrap();

        let commit_hash = commit_lore_message(&root, "chore: seed lore refs", true).unwrap();
        let ref_atom = LoreAtom {
            id: "sync-id-1".to_string(),
            kind: LoreKind::Decision,
            state: AtomState::Accepted,
            title: "Keep sync idempotent".to_string(),
            body: Some("Accepted from git history".to_string()),
            scope: Some("sync".to_string()),
            path: Some(PathBuf::from("src/git/mod.rs")),
            validation_script: None,
            created_unix_seconds: 10,
        };
        write_lore_ref(&root, &ref_atom, &commit_hash).unwrap();

        let first = sync_workspace_from_git_history(&root, &workspace).unwrap();
        let second = sync_workspace_from_git_history(&root, &workspace).unwrap();

        assert_eq!(first.len(), second.len());

        let unique_ids = second
            .iter()
            .map(|atom| atom.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(unique_ids.len(), second.len());
    }

    #[test]
    fn sync_workspace_compacts_existing_duplicate_atom_ids() {
        let root = std::env::temp_dir().join(format!("git-lore-sync-dedupe-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();

        let init_status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .status()
            .unwrap();
        assert!(init_status.success());

        let workspace = Workspace::init(&root).unwrap();
        let duplicate_id = "dup-1".to_string();

        workspace
            .set_state(&WorkspaceState {
                version: 1,
                atoms: vec![
                    LoreAtom {
                        id: duplicate_id.clone(),
                        kind: LoreKind::Decision,
                        state: AtomState::Proposed,
                        title: "Older duplicate".to_string(),
                        body: None,
                        scope: None,
                        path: None,
                        validation_script: None,
                        created_unix_seconds: 1,
                    },
                    LoreAtom {
                        id: duplicate_id.clone(),
                        kind: LoreKind::Decision,
                        state: AtomState::Accepted,
                        title: "Newer duplicate".to_string(),
                        body: Some("more complete".to_string()),
                        scope: Some("sync".to_string()),
                        path: Some(PathBuf::from("src/git/mod.rs")),
                        validation_script: None,
                        created_unix_seconds: 2,
                    },
                ],
            })
            .unwrap();

        let synced = sync_workspace_from_git_history(&root, &workspace).unwrap();

        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].id, duplicate_id);
        assert_eq!(synced[0].title, "Newer duplicate");
    }

    #[test]
    fn sync_workspace_preserves_existing_active_state_for_matching_refs() {
        let root = std::env::temp_dir().join(format!("git-lore-sync-preserve-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();

        let init_status = Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .status()
            .unwrap();
        assert!(init_status.success());

        let workspace = Workspace::init(&root).unwrap();
        let commit_hash = commit_lore_message(&root, "chore: seed lore refs", true).unwrap();

        let ref_atom = LoreAtom {
            id: "preserve-1".to_string(),
            kind: LoreKind::Decision,
            state: AtomState::Accepted,
            title: "Keep sync stable".to_string(),
            body: Some("Accepted from git history".to_string()),
            scope: Some("sync".to_string()),
            path: Some(PathBuf::from("src/git/mod.rs")),
            validation_script: None,
            created_unix_seconds: 10,
        };
        write_lore_ref(&root, &ref_atom, &commit_hash).unwrap();

        workspace
            .set_state(&WorkspaceState {
                version: 1,
                atoms: vec![LoreAtom {
                    id: "preserve-1".to_string(),
                    kind: LoreKind::Decision,
                    state: AtomState::Deprecated,
                    title: "Keep sync stable".to_string(),
                    body: Some("Resolved locally".to_string()),
                    scope: Some("sync".to_string()),
                    path: Some(PathBuf::from("src/git/mod.rs")),
                    validation_script: None,
                    created_unix_seconds: 20,
                }],
            })
            .unwrap();

        let synced = sync_workspace_from_git_history(&root, &workspace).unwrap();

        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].id, "preserve-1");
        assert_eq!(synced[0].state, AtomState::Deprecated);
        assert_eq!(synced[0].body.as_deref(), Some("Resolved locally"));
    }
}
