#[path = "../assimilation_authority.rs"]
mod assimilation_authority;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(assimilation_authority::run_assimilation_authority_guard(&args));
}
