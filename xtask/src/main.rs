use std::{
    env,
    process::{Command, Stdio},
};

const DOCKER_IMAGE: &str = "kirottu/cross:x86_64-unknown-freebsd";
const CROSS_DIR_PATH: &str = "ttc-bot/cross/docker";

const INFO_ANSI: &str = "\x1B[1:32m";
const CLEAR_ANSI: &str = "\x1B[0m";

macro_rules! match_command {
    ( $on_error:expr, $on_run_error:expr, $command:block ) => {
        match $command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    return Err($on_run_error.to_string());
                }
            }
            Err(why) => {
                return Err(format!("{}: {}", $on_error, why));
            }
        }
    };
}

macro_rules! match_task {
    ( $task:expr, $result:expr ) => {
        match $result {
            Ok(_) => (),
            Err(why) => {
                println!("Task \"{}\" returned with an Err value: {}", $task, why);
                return;
            }
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("No task provided!");
        return;
    }

    let task = args[1].clone();

    let project_dir = {
        let xtask_project_dir = env::var("CARGO_MANIFEST_DIR").expect("Not ran with cargo!");

        let mut xtask_project_dir_split: Vec<&str> = xtask_project_dir.split("/").collect();
        xtask_project_dir_split.pop();

        xtask_project_dir_split.join("/")
    };
    let cargo_home = env::var("CARGO_HOME").unwrap_or(format!(
        "{}/.cargo",
        env::var("HOME").expect("No $HOME variable set")
    ));
    let cargo = env::var("CARGO").expect("Could not determine cargo executable!");

    match &task[..] {
        "build-freebsd" => match_task!(task, build_freebsd(&project_dir, &cargo_home)),
        "build" => match_task!(task, build(&project_dir, &cargo)),
        "run" => match_task!(task, run(&project_dir, &cargo, &args[2..args.len()])),
        _ => {
            println!("Invalid task provided!");
            return;
        }
    }
}

fn build_freebsd(project_dir: &String, cargo_home: &String) -> Result<(), String> {
    if !{
        match Command::new("docker")
            .arg("images")
            .arg("-q")
            .arg(DOCKER_IMAGE)
            .output()
        {
            Ok(output) => {
                if output.stdout.len() > 0 {
                    true
                } else {
                    false
                }
            }
            Err(why) => {
                return Err(format!(
                    "Could not check for existing docker image: {}",
                    why
                ));
            }
        }
    } {
        println!(
            "{}Image not found locally, building...{}",
            INFO_ANSI, CLEAR_ANSI
        );
        match_command!(
            "Could not start docker build process",
            "Failed to build docker image",
            {
                Command::new("docker")
                    .arg("build")
                    .arg("-t")
                    .arg(DOCKER_IMAGE)
                    .arg(format!("{}/{}", project_dir, CROSS_DIR_PATH))
            }
        );
    }

    println!("{}Building ttc-bot...{}", INFO_ANSI, CLEAR_ANSI);
    match_command!(
        "Could not start the build process",
        "Failed to build ttc-bot",
        {
            Command::new(format!("{}/bin/cross", cargo_home))
                .current_dir(format!("{}/ttc-bot", project_dir))
                .env("CARGO_TERM_COLOR", "always")
                .arg("build")
                .arg("--target")
                .arg("x86_64-unknown-freebsd")
                .arg("--release")
        }
    );

    println!("{}Stripping the binary...{}", INFO_ANSI, CLEAR_ANSI);
    match_command!(
        "Could not start strip process",
        "Failed to strip the binary",
        {
            Command::new("strip")
                .current_dir(format!(
                    "{}/ttc-bot/target/x86_64-unknown-freebsd/release",
                    project_dir
                ))
                .arg("-s")
                .arg("./ttc-bot")
        }
    );

    Ok(())
}

fn build(project_dir: &String, cargo: &String) -> Result<(), String> {
    println!("{}Building ttc-bot...{}", INFO_ANSI, CLEAR_ANSI);
    match_command!(
        "Could not start build process",
        "Failed to build ttc-bot",
        {
            Command::new(cargo)
                .current_dir(format!("{}/ttc-bot", project_dir))
                .env("CARGO_TERM_COLOR", "always")
                .arg("build")
        }
    );

    Ok(())
}

fn run(project_dir: &String, cargo: &String, args: &[String]) -> Result<(), String> {
    println!("{}Running ttc-bot...{}", INFO_ANSI, CLEAR_ANSI);

    match_command!("Failed to start process", "Process failed", {
        Command::new(cargo)
            .current_dir(format!("{}/ttc-bot", project_dir))
            .env("CARGO_TERM_COLOR", "always")
            .arg("run")
            .arg("--")
            .args(args)
    });

    Ok(())
}
