use std::process::{Command, Output};

pub trait CommandExecutor {
    fn execute(&self, command: &str, args: &[&str]) -> std::io::Result<Output>;
    fn execute_with_env(
        &self,
        command: &str,
        args: &Vec<&str>,
        env: Vec<(&str, &str)>,
    ) -> std::io::Result<Output>;
    fn run_script_from_string(&self, script: &str) -> std::io::Result<Output>;
}

struct DefaultExecutor;

impl CommandExecutor for DefaultExecutor {
    fn execute(&self, command: &str, args: &[&str]) -> std::io::Result<Output> {
        Command::new(command).args(args).output()
    }
    fn execute_with_env(
        &self,
        command: &str,
        args: &Vec<&str>,
        env: Vec<(&str, &str)>,
    ) -> std::io::Result<Output> {
        let mut binding = Command::new(command);
        let mut command = binding.args(args);
        for (key, value) in env {
            command = command.env(key, value);
        }
        command.output()
    }
    fn run_script_from_string(&self, script: &str) -> std::io::Result<Output> {
        self.execute("bash", &["-c", script])
    }
}

#[cfg(target_os = "windows")]
struct WindowsExecutor;

#[cfg(target_os = "windows")]
impl CommandExecutor for WindowsExecutor {
    fn execute(&self, command: &str, args: &[&str]) -> std::io::Result<Output> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new(command)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
    }
    fn execute_with_env(
        &self,
        command: &str,
        args: &Vec<&str>,
        env: Vec<(&str, &str)>,
    ) -> std::io::Result<Output> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut binding = Command::new(command);
        let mut command = binding.args(args).creation_flags(CREATE_NO_WINDOW);
        for (key, value) in env {
            command = command.env(key, value);
        }
        command.output()
    }
    fn run_script_from_string(&self, script: &str) -> std::io::Result<Output> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut child = Command::new("powershell")
            .args(["-Command", "-"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        output
    }
}

pub fn get_executor() -> Box<dyn CommandExecutor> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsExecutor)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(DefaultExecutor)
    }
}

pub fn execute_command(command: &str, args: &[&str]) -> std::io::Result<Output> {
    let executor = get_executor();
    executor.execute(command, args)
}
