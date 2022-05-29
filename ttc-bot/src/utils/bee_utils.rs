use crate::utils::bee_script;
use lazy_static::lazy_static;
use rand::Rng;

lazy_static! {
    static ref BEE_LINE_VEC: Vec<String> = bee_lines();
}

fn bee_lines() -> Vec<String> {
    bee_script::BEE_SCRIPT
        .lines()
        .map(|line| line.to_string())
        .collect()
}

pub fn get_bee_line(index: Option<usize>) -> String {
    let index = match index {
        Some(index) => index,
        None => {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..BEE_LINE_VEC.len())
        }
    };
    BEE_LINE_VEC[index].clone()
}
