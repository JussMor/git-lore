use std::path::Path;
use anyhow::Result;

use memvid_core::{Memvid, PutOptions, SearchRequest};

use crate::lore::LoreAtom;

pub fn rebuild_index(workspace_root: &Path, atoms: &[LoreAtom], accepted: &[crate::lore::refs::AcceptedLoreRecord]) -> Result<()> {
    let index_dir = workspace_root.join(".lore");
    if !index_dir.exists() {
        std::fs::create_dir_all(&index_dir)?;
    }
    
    let db_path = index_dir.join("index.mv2");
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let mut mem = Memvid::create(&db_path)?;

    for atom in atoms {
        let text_content = format!("{} {}", atom.title, atom.body.as_deref().unwrap_or_default());
        let opts = PutOptions::builder()
            .title(&atom.title)
            .uri(&format!("mv2://active/{}", atom.id))
            .tag("state", &format!("{:?}", atom.state))
            .tag("scope", atom.scope.as_deref().unwrap_or_default())
            .tag("id", &atom.id)
            .build();
        mem.put_bytes_with_options(text_content.as_bytes(), opts)?;
    }

    for record in accepted {
        let text_content = format!("{} {}", record.atom.title, record.atom.body.as_deref().unwrap_or_default());
        let opts = PutOptions::builder()
            .title(&record.atom.title)
            .uri(&format!("mv2://archive/{}", record.atom.id))
            .tag("state", "AcceptedGroup")
            .tag("scope", record.atom.scope.as_deref().unwrap_or_default())
            .tag("id", &record.atom.id)
            .build();
        mem.put_bytes_with_options(text_content.as_bytes(), opts)?;
    }

    mem.commit()?;
    Ok(())
}

pub fn index_exists(workspace_root: &Path) -> bool {
    workspace_root.join(".lore").join("index.mv2").exists()
}

pub fn search(workspace_root: &Path, query: &str, limit: usize) -> Result<Vec<(String, f64, String)>> {
    let db_path = workspace_root.join(".lore").join("index.mv2");
    let mut mem = Memvid::open(&db_path)?;

    let search_response = mem.search(SearchRequest {
        query: query.to_string(),
        top_k: limit,
        snippet_chars: 200,
        uri: None,
        scope: None,
        cursor: None,
        as_of_frame: None,
        as_of_ts: None,
        no_sketch: false,
        acl_context: None,
        acl_enforcement_mode: memvid_core::types::AclEnforcementMode::Audit,
    })?;

    let mut results = Vec::new();
    for hit in search_response.hits {
        let mut id = "".to_string();
        let mut state_val = "".to_string();
        if let Some(meta) = &hit.metadata {
            if let Some(v) = meta.extra_metadata.get("id") {
                id = v.clone();
            }
            if let Some(v) = meta.extra_metadata.get("state") {
                state_val = v.clone();
            }
        }
        let score = hit.score.unwrap_or(0.0) as f64;
        results.push((id, score, state_val));
    }
    
    Ok(results)
}
