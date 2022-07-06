use lazy_static::lazy_static;
use serde::Serialize;
use std::collections::LinkedList;

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

#[derive(Serialize)]
pub enum TestingOutcome {
    Success,
    Timeout,
    MemoryExceeded,
    WrongOutput,
    SlightlyWrongOutput,
    InternalError,
}

#[derive(Serialize, Clone)]
pub enum TestLog {
    Success { time: u64, memory: f64 },
    Timeout { time_limit_millis: u64 },
    MemoryExceeded { memory_used: f64 },
    WrongOutput { expected: String, got: String },
    SlightlyWrongOutput { expected: String, got: String },
    InternalError(String),
}

impl TestLog {
    pub fn outcome(&self) -> TestingOutcome {
        match self {
            Self::Success { .. } => TestingOutcome::Success,
            Self::Timeout { .. } => TestingOutcome::Timeout,
            Self::MemoryExceeded { .. } => TestingOutcome::MemoryExceeded,
            Self::WrongOutput { .. } => TestingOutcome::WrongOutput,
            Self::SlightlyWrongOutput { .. } => TestingOutcome::SlightlyWrongOutput,
            Self::InternalError(..) => TestingOutcome::InternalError,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TestResult {
    test_id: u64,
    test_result: TestLog,
}

impl TestResult {
    pub fn new(test_id: u64, test_result: TestLog) -> TestResult {
        TestResult {
            test_id,
            test_result,
        }
    }
}

#[derive(Serialize)]
pub enum ProgramResult {
    CompilationProblem(String),
    InternalProblem(String),
    TestingResult {
        testing_outcome: TestingOutcome,
        tests: LinkedList<TestResult>,
    },
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
        serde_json::to_string(&testing_result).unwrap(),
    );
}
