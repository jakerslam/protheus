
fn thorn_cell_limit(state: &SwarmState) -> usize {
    (((state.sessions.len().max(1) as f64) * 0.10).ceil() as usize).max(1)
}
