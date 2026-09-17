#[cfg(test)]
mod tests {
    use crate::memory::MemoryStore;
    use crate::tools::AgentTool;
    use crate::memory_tools::{RememberPreferenceTool, RecallMemoryTool, RecordFixTool, RecordFileRelationTool};
    use serde_json::json;

    #[tokio::test]
    async fn test_memory_store_lifecycle() {
        let mut store = MemoryStore::default();
        let _ = store.set_preference("editor", "neovim");
        assert_eq!(store.get_preference("editor").unwrap(), "neovim");

        let _ = store.add_fix("lifetime error in async trait", "use BoxFuture and Pin");
        let fixes = store.search_fixes("lifetime error");
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].solution, "use BoxFuture and Pin");

        let _ = store.add_file_relation(
            "src/api.rs",
            "send_request",
            "calls",
            "src/tools.rs",
            "executes tool"
        );

        let rels = store.query_relations("send_request");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].target_file, "src/tools.rs");
    }

    #[tokio::test]
    async fn test_memory_tools_execution() {
        let pref_tool = RememberPreferenceTool;
        let _res1 = pref_tool.execute(json!({
            "key": "my_test_key",
            "value": "my_test_val"
        })).await;

        let recall_tool = RecallMemoryTool;
        let res2 = recall_tool.execute(json!({
            "query": "my_test_key"
        })).await;
        println!("RECALL RES: {}", res2);
        assert!(res2.contains("my_test_val"));

        let fix_tool = RecordFixTool;
        let res3 = fix_tool.execute(json!({
            "problem": "null pointer in unity component",
            "solution": "check if component is null before invoking"
        })).await;
        assert!(res3.contains("Successfully recorded past fix"));

        let rel_tool = RecordFileRelationTool;
        let res4 = rel_tool.execute(json!({
            "file_path": "src/main.rs",
            "related_symbol": "main",
            "relation_type": "depends_on",
            "target_file": "src/lib.rs",
            "notes": "initializes modules"
        })).await;
        assert!(res4.contains("Successfully recorded file and symbol relationship"));
    }
}
