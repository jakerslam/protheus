struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}
