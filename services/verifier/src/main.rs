use std::env;

fn main() {
    let path = env::args().nth(1).unwrap_or_default();
    let out = verifier::verify_with_env(&path)
        .unwrap_or_else(|e| verifier::VerifyOutput::with_error(path, format!("{:#}", e)));
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    if out.error.is_some() {
        std::process::exit(1);
    }
}
