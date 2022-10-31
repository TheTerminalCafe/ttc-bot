use std::process::Command;
use std::str;

fn main() {
    let output = Command::new("git")
        .arg("rev-list")
        .arg("HEAD")
        .output()
        .expect("Failed to run git command to get latest commit")
        .stdout;
    let output = str::from_utf8(&output).unwrap();
    let index = output.find('\n').unwrap();
    println!("cargo:rustc-env=CURRENT_COMMIT={}", &output[..index]);
}
