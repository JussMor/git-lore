use super::LoreAtom;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SanitizationIssue {
    pub atom_id: String,
    pub field: String,
    pub reason: String,
}

pub fn scan_atoms(atoms: &[LoreAtom]) -> Vec<SanitizationIssue> {
    let mut issues = Vec::new();

    for atom in atoms {
        if let Some(reason) = scan_text(&atom.title) {
            issues.push(SanitizationIssue {
                atom_id: atom.id.clone(),
                field: "title".to_string(),
                reason,
            });
        }

        if let Some(body) = atom.body.as_deref() {
            if let Some(reason) = scan_text(body) {
                issues.push(SanitizationIssue {
                    atom_id: atom.id.clone(),
                    field: "body".to_string(),
                    reason,
                });
            }
        }
    }

    issues
}

pub fn scan_text(text: &str) -> Option<String> {
    let lowered = text.to_ascii_lowercase();

    let markers = [
        ("api key", "possible API key disclosure"),
        ("secret", "possible secret disclosure"),
        ("token", "possible token disclosure"),
        ("password", "possible password disclosure"),
        ("private key", "possible private key disclosure"),
        ("-----begin", "possible private key block"),
        ("akia", "possible AWS key disclosure"),
        ("xoxb-", "possible Slack token disclosure"),
        ("ghp_", "possible GitHub token disclosure"),
        ("AIza", "possible Google API key disclosure"),
    ];

    markers
        .iter()
        .find(|(needle, _)| lowered.contains(*needle))
        .map(|(_, reason)| reason.to_string())
}