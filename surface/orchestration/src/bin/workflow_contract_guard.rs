use infring_orchestration_surface_v1::control_plane::workflow_contract_guard::run_workflow_contract_guard;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    ExitCode::from(run_workflow_contract_guard(&args) as u8)
}
