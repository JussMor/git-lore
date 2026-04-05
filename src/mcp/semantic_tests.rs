use crate::lore::LoreAtom;
use tempfile::tempdir;
use crate::lore::AtomState;
#[test]
fn test_index_rebuild_and_search() {
    let dir = tempdir().unwrap();
    let atom1 = LoreAtom {
        id: "1".into(),
        created_unix_seconds: 0,
        state: AtomState::Accepted,
        kind: "code_style".into(),
        title: "Use JSON over XML".into(),
        body: Some("Always prefer JSON encoding for logs".into()),
        scope: None,
        path: None,
        prism: vec![],
        validation_script: None,
    };
    crate::mcp::semantic::rebuild_index(dir.path(), &[atom1], &[]).unwrap();
    assert!(crate::mcp::semantic::index_exists(dir.path()));
    let results = crate::mcp::semantic::search(dir.path(), "JSON encoding logs", 5).unwrap();
    assert!(!results.is_empty(), "Expected results from semantic search");
    assert_eq!(results[0].0, "1", "Should find atom 1");
}
