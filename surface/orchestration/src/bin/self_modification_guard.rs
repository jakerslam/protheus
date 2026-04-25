#[path = "../self_modification.rs"]
mod self_modification;
#[path = "../trust_zones.rs"]
#[allow(dead_code)]
mod trust_zones;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(self_modification::run_self_modification_guard(&args));
}
