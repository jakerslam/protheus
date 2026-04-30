#[path = "../continuous_eval.rs"]
mod continuous_eval;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(continuous_eval::run_continuous_eval(&args));
}
