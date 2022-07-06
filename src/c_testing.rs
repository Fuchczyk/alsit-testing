use std::{
    collections::LinkedList,
    fmt::Display,
    io::{Read, Write},
    path::PathBuf,
    process::Command,
    process::Stdio,
    time::Duration,
};
use wait_timeout::ChildExt;

use crate::{ProgramResult, TestLog, TestResult, TestingOutcome};

enum CompilationResult {
    Successful,
    CompilationError(String),
}

enum TestError {
    WritingStdin,
    SignalKill,
    ReadingStdout,
}

impl Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::WritingStdin => "Problem while writing to stdin.",
            Self::SignalKill => "Program killed by signal.",
            Self::ReadingStdout => "Problem while reading from stdout.",
        };

        write!(f, "{text}")
    }
}

pub fn invoke_testing() -> ProgramResult {
    // Compilation process and json result.
    match compile() {
        Ok(CompilationResult::Successful) => {}
        Ok(CompilationResult::CompilationError(error)) => {
            return ProgramResult::CompilationProblem(error);
        }
        Err(error) => {
            println!("ERROR COMPILATION = {}", error);
            return ProgramResult::InternalProblem(error);
        }
    }

    match run_testing() {
        Err(error) => ProgramResult::InternalProblem(error),
        Ok((list, outcome)) => ProgramResult::TestingResult {
            testing_outcome: outcome,
            tests: list,
        },
    }
}

fn analyse_result(expected: String, outcome: String, time: u64, memory: f64) -> TestLog {
    if expected == outcome {
        return TestLog::Success { time, memory };
    }

    if expected.trim() == outcome {
        return TestLog::SlightlyWrongOutput {
            expected,
            got: outcome,
        };
    }

    TestLog::WrongOutput {
        expected,
        got: outcome,
    }
}

// TODO: ERROR PROOF
fn test(in_file: &PathBuf, out_file: &PathBuf) -> Result<TestLog, TestError> {
    let in_content = std::fs::read_to_string(in_file).unwrap();
    let out_content = std::fs::read_to_string(out_file).unwrap();

    let mut process_spawn = Command::new(format!(
        "{}{}",
        crate::PROGRAM_PATH,
        crate::COMPILED_PROGRAM_NAME
    ))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();

    match process_spawn
        .stdin
        .as_mut()
        .unwrap()
        .write_all(in_content.as_bytes())
    {
        Ok(()) => {}
        Err(_) => {
            return Err(TestError::WritingStdin);
        }
    };

    match process_spawn
        .wait_timeout(Duration::from_millis(*crate::TESTING_TIMEOUT_TIME_MILLS))
        .unwrap()
    {
        Some(status) => {
            if status.code().is_some() {
                let mut output = String::new();

                match process_spawn.stdout.unwrap().read_to_string(&mut output) {
                    Err(_) => Err(TestError::ReadingStdout),
                    Ok(_) => Ok(analyse_result(out_content, output, 0, 0.0)),
                }
            } else {
                Err(TestError::SignalKill)
            }
        }
        None => {
            let _ = process_spawn.kill();
            Ok(TestLog::Timeout {
                time_limit_millis: *crate::TESTING_TIMEOUT_TIME_MILLS,
            })
        }
    }
}

fn get_id(path: &std::path::Path) -> u64 {
    let mut path_c = path.to_path_buf();
    let _ = path_c.set_extension("");

    path_c
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap()
}

fn run_testing() -> Result<(LinkedList<TestResult>, TestingOutcome), String> {
    let files = match std::fs::read_dir(crate::TEST_PATH) {
        Ok(res) => res,
        Err(_) => {
            return Err("Error while scanning directory.".into());
        }
    };

    let mut in_files = Vec::new();
    for file in files {
        let path = file.unwrap().path();

        match path.extension() {
            None => {}
            Some(ext) => {
                if ext.eq("in") {
                    in_files.push(path);
                }
            }
        }
    }

    in_files.sort_by(|x, y| {
        let x_name: u64 = get_id(x);

        let y_name: u64 = get_id(y);

        x_name.cmp(&y_name)
    });

    let mut list: LinkedList<TestResult> = LinkedList::new();

    for file in in_files {
        let in_path = file;

        let mut out_path = in_path.clone();
        let _ = out_path.set_extension("out");

        let test_id: u64 = get_id(&in_path);

        match test(&in_path, &out_path) {
            Err(error) => {
                list.push_back(TestResult::new(
                    test_id,
                    TestLog::InternalError(error.to_string()),
                ));
                return Ok((list, TestingOutcome::InternalError));
            }
            Ok(TestLog::Success { time, memory }) => {
                list.push_back(TestResult::new(test_id, TestLog::Success { time, memory }));
            }
            Ok(result) => {
                list.push_back(TestResult::new(test_id, result.clone()));
                return Ok((list, result.outcome()));
            }
        }
    }

    Ok((list, TestingOutcome::Success))
}

fn compile() -> Result<CompilationResult, String> {
    let process_output = Command::new("gcc")
        .arg("-O2")
        .arg(format!("{}{}", crate::PROGRAM_PATH, crate::PROGRAM_NAME))
        .arg("-o")
        .arg(format!(
            "{}{}",
            crate::PROGRAM_PATH,
            crate::COMPILED_PROGRAM_NAME
        ))
        .output();

    let output = match process_output {
        Ok(o) => o,
        Err(_) => {
            return Err("Internal error occured while starting compilation process.".into());
        }
    };

    if let Some(code) = output.status.code() {
        if code == 0 {
            Ok(CompilationResult::Successful)
        } else {
            let comunicate = match String::from_utf8(output.stderr) {
                Ok(result) => result,
                Err(_) => {
                    return Err(
                        "Compilation message couldn't be converted into UTF-8 string.".into(),
                    )
                }
            };

            Ok(CompilationResult::CompilationError(comunicate))
        }
    } else {
        Err("Compilation process terminated by sginal.".into())
    }
}
