#[cfg(test)]
mod tests {
    use crate::memory::MemoryStore;
    use crate::memory_tools::{RecallMemoryTool, RecordFileRelationTool, RecordFixTool};
    use crate::tools::AgentTool;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, MutexGuard};

    // RUNE_MEMORY_PATH is process-global, and cargo runs tests in parallel.
    // Every test that touches it must hold this lock for its whole duration.
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Takes the env lock, points RUNE_MEMORY_PATH at a fresh temp dir, and
    /// restores everything on drop (even if the test panics).
    struct TestEnv {
        dir: PathBuf,
        _lock: MutexGuard<'static, ()>,
    }

    impl TestEnv {
        fn new() -> Self {
            // Recover from poisoning so one failed test doesn't fail the others.
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!(
                "rune_test_{}_{}_{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst),
                nanos
            ));
            std::fs::create_dir_all(&dir).expect("failed to create temp dir");

            unsafe {
                std::env::set_var("RUNE_MEMORY_PATH", dir.join("memory.json"));
            }

            TestEnv { dir, _lock: lock }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("RUNE_MEMORY_PATH");
            }
            let _ = std::fs::remove_dir_all(&self.dir);
            // Fields drop after this, so the lock is released only once the env is clean.
        }
    }

    #[test]
    fn test_memory_store_lifecycle() {
        let _env = TestEnv::new();

        let mut store = MemoryStore::default();

        store.set_preference("editor", "neovim").unwrap();
        assert_eq!(store.get_preference("editor").unwrap(), "neovim");

        store
            .add_fix("lifetime error in async trait", "use BoxFuture and Pin")
            .unwrap();
        let fixes = store.search_fixes("lifetime error");
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].solution, "use BoxFuture and Pin");

        store
            .add_file_relation(
                "src/api.rs",
                "send_request",
                "calls",
                "src/tools.rs",
                "executes tool",
            )
            .unwrap();
        let rels = store.query_relations("send_request");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].target_file, "src/tools.rs");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_find_fix_for_error() {
        let _env = TestEnv::new();
        let mut store = MemoryStore::default();
        store
            .add_fix("error[E0308]: mismatched types", "convert i32 to u32 using as u32")
            .unwrap();

        let loaded = MemoryStore::load();
        let fix = loaded.find_fix_for_error("error[E0308]: mismatched types found in expression");
        assert!(fix.is_some());
        let f = fix.unwrap();
        assert_eq!(f.solution, "convert i32 to u32 using as u32");

        let no_fix = loaded.find_fix_for_error("some completely unrelated error");
        assert!(no_fix.is_none());
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_memory_tools_execution() {
        let _env = TestEnv::new();

        let mut initial_store = MemoryStore::default();
        initial_store
            .set_preference("my_test_key", "my_test_val")
            .unwrap();

        let recall_tool = RecallMemoryTool;
        let res = recall_tool.execute(json!({ "query": "my_test_key" })).await;
        assert!(res.contains("my_test_val"), "recall result: {res}");

        let fix_tool = RecordFixTool;
        let res = fix_tool
            .execute(json!({
                "problem": "null pointer in unity component",
                "solution": "check if component is null before invoking"
            }))
            .await;
        assert!(
            res.contains("Successfully recorded past fix"),
            "fix result: {res}"
        );

        let rel_tool = RecordFileRelationTool;
        let res = rel_tool
            .execute(json!({
                "file_path": "src/main.rs",
                "related_symbol": "main",
                "relation_type": "depends_on",
                "target_file": "src/lib.rs",
                "notes": "initializes modules"
            }))
            .await;
        assert!(
            res.contains("Successfully recorded file and symbol relationship"),
            "relation result: {res}"
        );
    }
}