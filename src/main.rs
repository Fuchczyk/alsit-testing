use lazy_static::lazy_static;

mod c_testing;

const RESULT_PATH: &str = "/output/";
const OUTPUT_NAME: &str = "result.json";
const TEST_PATH: &str = "/tests/";
const PROGRAM_PATH: &str = "/program/";
const PROGRAM_NAME: &str = "main.c";
const COMPILED_PROGRAM_NAME: &str = "compiled_program";

lazy_static! {
    static ref TESTING_TIMEOUT_TIME_MILLS: u64 = {
        std::env::var("TIMEOUT_TIME")
            .unwrap()
            .parse::<u64>()
            .expect("Unable to parse TIMEOUT_TIME into 64 bit unsigned int.")
    };
}

fn main() {
    //TODO: Make other languages than c
    let test_language = std::env::var("TEST_LANGUAGE").unwrap();

    let testing_result = {
        if test_language == "C" {
            c_testing::invoke_testing()
        } else {
            panic!("Language support is not provided yet.");
        }
    };

    let _ = std::fs::write(
        format!("{}{}", crate::RESULT_PATH, crate::OUTPUT_NAME),
        testing_result,
    );
}
