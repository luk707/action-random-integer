mod utils;

use core::panic;
use rand::Rng;
use std::io::{self, BufRead};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use utils::{
    Data::{Input, Stream},
    StreamProcessor,
};

#[derive(Default, Serialize, Deserialize, Debug)]
struct RandomIntegerInput {
    min: i32,
    max: i32,
}

pub fn main() -> Result<(), std::io::Error> {
    let stdin = io::stdin();
    let handle = stdin.lock();
    let mut processor = StreamProcessor::<RandomIntegerInput>::new();

    for line in handle.lines() {
        let line = line?;
        let json: Value = serde_json::from_str(&line)?;

        for data in processor.process(json) {
            match data {
                Input(input) => {
                    println!("Input: {:?}", input);
                    if input.min > input.max {
                        panic!("Min is greater than max");
                    }
                    let mut rng = rand::thread_rng();
                    let result = rng.gen_range(input.min..=input.max);
                    let output = serde_json::json!({
                        "result": result,
                    });
                    println!("Output: {}", output);
                    outputjson!(&output);
                }
                Stream(key, value) => {
                    panic!("Unexpected stream data: {} {:?}", key, value);
                }
                _ => {}
            }
        }
    }
    Ok(())
}
