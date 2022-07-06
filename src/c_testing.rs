use serde::Serialize;
use std::{
    collections::LinkedList,
    io::{Read, Write},
    path::PathBuf,
    process::Command,
    process::Stdio,
    time::Duration,
};
use wait_timeout::ChildExt;

enum CompilationResult {
    Successful,
    CompilationError(String),
}

#[derive(Serialize)]
enum TestOutcome {
    Success,
    Timeout,
    WrongOutput { expected: String, got: String },
    SlightlyWrongOutput { expected: String, got: String },
    InternalError,
}

enum TestError {
    WritingStdin,
    SignalKill,
    ReadingStdout,
}

#[derive(Serialize)]
struct InitialError {
    compilation_message: Option<String>,
    internal_error: Option<String>,
}

#[derive(Serialize)]
struct OneTestResult {
    test_id: u64,
    test_result: TestOutcome, // TODO: Memory and TIME
}

impl OneTestResult {
    fn new(test_id: u64, test_result: TestOutcome) -> OneTestResult {
        OneTestResult {
            test_id,
            test_result,
        }
    }
}

pub fn invoke_testing() -> String {
    // Compilation process and json result.
    match compile() {
        Ok(CompilationResult::Successful) => {}
        Ok(CompilationResult::CompilationError(error)) => {
            return serde_json::to_string_pretty(&InitialError {
                compilation_message: Some(error),
                internal_error: None,
            })
            .unwrap();
        }
        Err(error) => {
            println!("ERROR COMPILATION = {}", error);
            return serde_json::to_string(&InitialError {
                compilation_message: None,
                internal_error: Some(error),
            })
            .unwrap();
        }
    }

    match run_testing() {
        Err(error) => serde_json::to_string(&InitialError {
            compilation_message: None,
            internal_error: Some(error),
        })
        .unwrap(),
        Ok(list) => serde_json::to_string(&list).unwrap(),
    }
}

fn diff_result(expected: String, outcome: String) -> TestOutcome {
    if expected == outcome {
        return TestOutcome::Success;
    }

    if expected.trim() == outcome {
        return TestOutcome::SlightlyWrongOutput {
            expected,
            got: outcome,
        };
    }

    TestOutcome::WrongOutput {
        expected,
        got: outcome,
    }
}

// TODO: ERROR PROOF
fn test(in_file: &PathBuf, out_file: &PathBuf) -> Result<TestOutcome, TestError> {
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
                    Ok(_) => Ok(diff_result(out_content, output)),
                }
            } else {
                Err(TestError::SignalKill)
            }
        }
        None => {
            let _ = process_spawn.kill();
            Ok(TestOutcome::Timeout)
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

fn run_testing() -> Result<LinkedList<OneTestResult>, String> {
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

    let mut list: LinkedList<OneTestResult> = LinkedList::new();

    for file in in_files {
        let in_path = file;

        let mut out_path = in_path.clone();
        let _ = out_path.set_extension("out");

        let test_id: u64 = get_id(&in_path);

        match test(&in_path, &out_path) {
            Err(_) => {
                list.push_back(OneTestResult::new(test_id, TestOutcome::InternalError));
                break;
            }
            Ok(TestOutcome::Success) => {
                list.push_back(OneTestResult::new(test_id, TestOutcome::Success));
            }
            Ok(result) => {
                list.push_back(OneTestResult::new(test_id, result));
                break;
            }
        }
    }

    Ok(list)
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
