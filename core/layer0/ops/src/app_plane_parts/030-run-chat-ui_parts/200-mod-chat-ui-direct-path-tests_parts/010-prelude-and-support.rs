    use super::*;
    use std::fs;

    fn write_chat_script(root: &Path, payload: &Value) {
        let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
        let parent = path.parent().expect("chat script parent");
        fs::create_dir_all(parent).expect("mkdir chat script");
        fs::write(path, serde_json::to_string_pretty(payload).expect("chat script json"))
            .expect("write chat script");
    }
