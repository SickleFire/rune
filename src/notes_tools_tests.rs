use super::*;
use crate::memory::FileRelation;
use std::fs;

#[test]
fn recall_ranks_links_tags_keywords_and_memory_relations() {
    let temp = tempfile::tempdir().expect("create notes test directory");
    let notes_dir = temp.path();
    fs::create_dir_all(notes_dir.join("concepts")).unwrap();
    fs::create_dir_all(notes_dir.join("src")).unwrap();
    fs::write(
        notes_dir.join("concepts/alpha.md"),
        "---\ntitle: \"Alpha Graph\"\ntags: [architecture, graph]\n---\nSee [[concepts/linked.md|Linked]]. Uses `src/a.rs`.",
    )
    .unwrap();
    fs::write(
        notes_dir.join("concepts/linked.md"),
        "---\ntitle: \"Linked Graph\"\ntags: [architecture]\n---\nA related node.",
    )
    .unwrap();
    fs::write(
        notes_dir.join("src/b.rs.md"),
        "---\ntitle: \"Beta Module\"\n---\nThe beta implementation.",
    )
    .unwrap();

    let memory = MemoryStore {
        file_relations: vec![FileRelation {
            file_path: "src/a.rs".to_string(),
            related_symbol: "alpha".to_string(),
            relation_type: "depends_on".to_string(),
            target_file: "src/b.rs".to_string(),
            notes: "test relation".to_string(),
        }],
        ..MemoryStore::default()
    };
    let results = recall_related_notes(notes_dir, "concepts/alpha.md", &memory, 10).unwrap();

    assert_eq!(results[0].path, "concepts/linked.md");
    assert!(
        results[0]
            .reasons
            .iter()
            .any(|reason| reason == "wiki link")
    );
    assert!(
        results[0]
            .reasons
            .iter()
            .any(|reason| reason.starts_with("shared tags"))
    );
    assert!(
        results[0]
            .reasons
            .iter()
            .any(|reason| reason.starts_with("keyword overlap"))
    );
    let related_file = results
        .iter()
        .find(|result| result.path == "src/b.rs.md")
        .expect("memory-related source note");
    assert!(
        related_file
            .reasons
            .iter()
            .any(|reason| reason == "memory relation (depends_on)")
    );
}

#[test]
fn recall_rejects_paths_outside_notes_root() {
    let temp = tempfile::tempdir().expect("create notes test directory");
    let result = recall_related_notes(temp.path(), "../outside.md", &MemoryStore::default(), 10);
    assert!(result.unwrap_err().contains("Access denied"));
}
