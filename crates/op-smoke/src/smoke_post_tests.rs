use super::*;

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("op-smoke-post-{}-{id}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn checkpoint_cannot_alias_the_final_output_or_replace_an_input() {
    let dir = TestDir::new();
    let final_path = dir.0.join("final.op");
    let alias = dir.0.join(".").join("final.op");
    assert!(validate_checkpoint(&alias, Some(&final_path)).is_err());
    assert!(!final_path.exists());

    let input = dir.0.join("input.op");
    std::fs::write(&input, b"original input").unwrap();
    assert!(save_checkpoint(&EditorState::new(), &input, Some(&final_path)).is_err());
    assert_eq!(std::fs::read(&input).unwrap(), b"original input");
}

#[cfg(unix)]
#[test]
fn dangling_final_symlink_cannot_overwrite_a_new_checkpoint() {
    let dir = TestDir::new();
    let checkpoint = dir.0.join("pre-images.op");
    let final_path = dir.0.join("final.op");
    std::os::unix::fs::symlink(&checkpoint, &final_path).unwrap();
    assert!(matches!(
        save_checkpoint(&EditorState::new(), &checkpoint, Some(&final_path)),
        Err(PostGenerationError::UnresolvedSymlink(_))
    ));
    assert!(!checkpoint.exists());
}

#[test]
fn checkpoint_is_a_separate_canonical_document_and_cannot_be_reused() {
    let dir = TestDir::new();
    let checkpoint = dir.0.join("pre-images.op");
    let final_path = dir.0.join("final.op");
    let state = EditorState::starter();
    save_checkpoint(&state, &checkpoint, Some(&final_path)).unwrap();
    let original = std::fs::read(&checkpoint).unwrap();
    assert_eq!(original, crate::smoke_save::bytes(&state).unwrap());
    assert!(!final_path.exists());
    assert!(save_checkpoint(&state, &checkpoint, Some(&final_path)).is_err());
    assert_eq!(std::fs::read(&checkpoint).unwrap(), original);
}

#[test]
fn expired_image_stage_saves_both_artifacts_and_exits_incomplete() {
    let dir = TestDir::new();
    let checkpoint = dir.0.join("pre-images.op");
    let final_path = dir.0.join("final.op");
    let mut state = EditorState::new();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "image", "id": "photo", "src": "", "imageSearchQuery": "coffee cup",
            "width": 56, "height": 56
        }))
        .unwrap(),
    );
    let post = PostGeneration {
        images: ImageFillConfig {
            enabled: true,
            limit: 40,
            delay_ms: 0,
            // The public parser refuses zero; this injects an already-expired
            // deadline to test persistence without any network request.
            budget: Some(std::time::Duration::ZERO),
        },
        output: Some(final_path.clone()),
        checkpoint: Some(checkpoint.clone()),
    };
    assert_eq!(post.run(&mut state, false), 5);
    for path in [&checkpoint, &final_path] {
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(saved["children"][0]["id"], "photo");
    }
    assert_eq!(
        std::fs::read(&checkpoint).unwrap(),
        crate::smoke_save::bytes(&state).unwrap()
    );
}
