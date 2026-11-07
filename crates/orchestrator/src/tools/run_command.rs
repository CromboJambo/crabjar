use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err(
            "Error: run_command requires at least 2 arguments (tool and args)".to_string(),
        );
    }
    let tool = &args[0];
    let command_args = &args[1..];

    let mut child = match tokio::process::Command::new(tool)
        .args(command_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return Err(format!("Error spawning command: {}", e)),
    };

    let stdout = child.stdout.take().expect("Failed to take stdout");
    let stderr = child.stderr.take().expect("Failed to take stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut output = String::new();

    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        output.push_str(&l);
                        output.push('\n');
                    }
                    Ok(None) => {}
                    Err(e) => {
                        output.push_str(&format!("Error reading stdout: {}", e));
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        output.push_str(&format!("stderr: {}\n", l));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        output.push_str(&format!("Error reading stderr: {}", e));
                        break;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        output.push_str(&format!("\nExit code: {}", exit_status));
                    }
                    Err(e) => {
                        output.push_str(&format!("\nError waiting for process: {}", e));
                    }
                }
                break;
            }
        }
    }

    Ok(output)
}
