use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{AtomState, LoreAtom};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub atom_id: String,
    pub command: String,
    pub reason: String,
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