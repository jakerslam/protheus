const RUNTIME_WEB_TOOLS_METADATA_REL: &str =
    "client/runtime/local/state/web_conduit/runtime_web_tools_metadata.json";
const WEB_RECEIPTS_REL: &str = "client/runtime/local/state/web_conduit/receipts.jsonl";

fn runtime_web_tools_metadata_path(root: &Path) -> PathBuf {
    runtime_state_path(root, RUNTIME_WEB_TOOLS_METADATA_REL)
}
