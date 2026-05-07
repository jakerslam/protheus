    use super::*;
    use std::fs;
    use std::path::Path;

    fn write_chat_script(root: &Path, payload: &Value) {
        let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
        let parent = path.parent().expect("chat script parent");
        fs::create_dir_all(parent).expect("mkdir chat script");
        fs::write(path, serde_json::to_string_pretty(payload).expect("chat script json"))
            .expect("write chat script");
    }

    fn write_chat_settings(root: &Path, provider: &str, model: &str) {
        let path = chat_ui_settings_path(root);
        let parent = path.parent().expect("chat settings parent");
        fs::create_dir_all(parent).expect("mkdir chat settings");
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "provider": provider,
                "model": model,
                "updated_at": crate::now_iso()
            }))
            .expect("chat settings json"),
        )
        .expect("write chat settings");
    }
