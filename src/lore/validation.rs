use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{AtomState, LoreAtom};

const NARRATIVE_PREFIXES: &[&str] = &[
    "verify that",
    "ensure that",
    "check that",
    "confirm that",
    "make sure",
    "make sure that",
    "validate that",
    "describe how",
    "explain how",
    "the script should",
    "this script should",
    "the command should",
    "this command should",
    "comprueba que",
    "verifica que",
    "asegúrate de",
    "asegure que",
    "asegúrese de",
    "confirma que",
    "describe cómo",
    "explica cómo",
    "escribe un",
    "escribe una",
    "valida que",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub atom_id: String,
    pub command: String,
    pub reason: String,
}

pub fn validate_script(script: &str) -> Result<()> {
    let script = script.trim();

    if script.is_empty() {
        bail!("validation script cannot be empty");
    }

    if script.contains('\n') || script.contains('\r') || script.contains('\0') {
        bail!("validation script must be a single-line literal shell command");
    }

    if looks_like_narrative(script) {
        bail!("validation script looks like natural language; provide a literal shell command instead");
    }

    Ok(())
}

pub fn scan_atoms(workspace_root: &Path, atoms: &[LoreAtom]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for atom in atoms.iter().filter(|atom| atom.state != AtomState::Deprecated) {
        let Some(script) = atom.validation_script.as_deref().map(str::trim) else {
            continue;
        };

        if script.is_empty() {
            continue;
        }

        if let Err(reason) = validate_script(script) {
            issues.push(ValidationIssue {
                atom_id: atom.id.clone(),
                command: script.to_string(),
                reason: reason.to_string(),
            });
            continue;
        }

        match run_script(workspace_root, atom, script) {
            Ok(()) => {}
            Err(reason) => issues.push(ValidationIssue {
                atom_id: atom.id.clone(),
                command: script.to_string(),
                reason: reason.to_string(),
            }),
        }
    }

    issues
}

fn run_script(workspace_root: &Path, atom: &LoreAtom, script: &str) -> Result<()> {
    let output = Command::new("/bin/sh")
        .arg("-lc")
        .arg(script)
        .current_dir(workspace_root)
        .env("GIT_LORE_ATOM_ID", &atom.id)
        .env("GIT_LORE_ATOM_KIND", format!("{:?}", atom.kind))
        .env("GIT_LORE_ATOM_STATE", format!("{:?}", atom.state))
        .env("GIT_LORE_ATOM_TITLE", &atom.title)
        .env("GIT_LORE_ATOM_SCOPE", atom.scope.as_deref().unwrap_or(""))
        .env(
            "GIT_LORE_ATOM_PATH",
            atom.path.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
        )
        .output()
        .with_context(|| format!("failed to run validation script for atom {}", atom.id))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if stderr.is_empty() && stdout.is_empty() {
        format!("validation script exited with status {}", output.status)
    } else if stderr.is_empty() {
        format!("validation script failed: {stdout}")
    } else if stdout.is_empty() {
        format!("validation script failed: {stderr}")
    } else {
        format!("validation script failed: {stderr}; {stdout}")
    };

    Err(anyhow::anyhow!(details))
}

fn looks_like_narrative(script: &str) -> bool {
    let lowered = script.to_ascii_lowercase();

    NARRATIVE_PREFIXES
        .iter()
        .any(|prefix| lowered.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use crate::lore::{AtomState, LoreAtom, LoreKind, Workspace};

    use super::{scan_atoms, validate_script};

    #[test]
    fn validate_script_accepts_literal_shell_command() {
        validate_script("cargo test --quiet").unwrap();
    }

    #[test]
    fn validate_script_rejects_narrative_text() {
        let error = validate_script("Verify that external writes are blocked before commit")
            .unwrap_err();

        assert!(error.to_string().contains("literal shell command"));
    }

    #[test]
    fn scan_atoms_reports_narrative_scripts_without_shelling_out() {
        let temp_root = std::env::temp_dir().join(format!(
            "git-lore-validation-test-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Guard writes".to_string(),
            None,
            None,
            Some(PathBuf::from("src/lib.rs")),
        )
        .with_validation_script(Some(
            "Verify that external writes are blocked before commit".to_string(),
        ));

        let issues = scan_atoms(&temp_root, &[atom]);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].command, "Verify that external writes are blocked before commit");
        assert!(issues[0].reason.contains("literal shell command"));
    }

    #[test]
    fn workspace_rejects_narrative_validation_script_on_record() {
        let temp_root = std::env::temp_dir().join(format!(
            "git-lore-validation-record-test-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Guard writes".to_string(),
            None,
            None,
            None,
        )
        .with_validation_script(Some(
            "Verify that external writes are blocked before commit".to_string(),
        ));

        let error = workspace.record_atom(atom).unwrap_err();
        assert!(error.to_string().contains("literal shell command"));
    }
}