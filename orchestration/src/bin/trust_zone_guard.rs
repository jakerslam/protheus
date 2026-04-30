#[path = "../trust_zones.rs"]
#[allow(dead_code)]
mod trust_zones;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(trust_zones::run_trust_zone_guard(&args));
}
