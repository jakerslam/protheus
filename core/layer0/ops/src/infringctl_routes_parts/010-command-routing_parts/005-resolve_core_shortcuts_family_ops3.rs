fn resolve_core_shortcuts_family_ops3(cmd: &str, rest: &[String]) -> Option<Route> {
    resolve_core_shortcuts_family_ops3_group_1(cmd, rest)
        .or_else(|| resolve_core_shortcuts_family_ops3_group_2(cmd, rest))
        .or_else(|| resolve_core_shortcuts_family_ops3_group_3(cmd, rest))
}

include!("005-resolve_core_shortcuts_family_ops3_fn_parts/001-resolve_core_shortcuts_family_ops3_group_1.rs");
include!("005-resolve_core_shortcuts_family_ops3_fn_parts/002-resolve_core_shortcuts_family_ops3_group_2.rs");
include!("005-resolve_core_shortcuts_family_ops3_fn_parts/003-resolve_core_shortcuts_family_ops3_group_3.rs");
