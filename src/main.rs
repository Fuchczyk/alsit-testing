mod c_testing;

const RESULT_PATH: &str = "/output/";
const OUTPUT_NAME: &str = "result.json";
const TEST_PATH: &str = "/tests/";
const PROGRAM_PATH: &str = "/program/";
const PROGRAM_NAME: &str = "main.c";
const COMPILED_PROGRAM_NAME: &str = "compiled_program";
const TESTING_TIMEOUT_TIME_SECS: u8 = env!("TIMEOUT_TIME");

fn main() {
    //TODO: Make other languages than c
    let testing_result = c_testing::invoke_testing();
    let _ = std::fs::write(format!("{}{}", crate::RESULT_PATH, crate::OUTPUT_NAME), testing_result);
}
