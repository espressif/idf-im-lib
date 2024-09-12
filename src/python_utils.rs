use log::trace;
use rustpython_vm as vm;
use rustpython_vm::function::PosArgs;
use std::env;
use std::process::ExitCode;
use vm::{builtins::PyStrRef, Interpreter};

/// Runs a Python script from a specified file with optional arguments and environment variables.
///
/// # Parameters
///
/// * `path` - A reference to a string representing the path to the Python script file.
/// * `args` - An optional reference to a string representing the arguments to be passed to the Python script.
/// * `python` - An optional reference to a string representing the Python interpreter to be used.
/// * `envs` - An optional reference to a vector of tuples representing environment variables to be set for the Python script.
///
/// # Returns
///
/// * `Result<String, String>` - On success, returns a `Result` containing the standard output of the Python script as a string.
///   On error, returns a `Result` containing the standard error of the Python script as a string.
pub fn run_python_script_from_file(
    path: &str,
    args: Option<&str>,
    python: Option<&str>,
    envs: Option<&Vec<(String, String)>>,
) -> Result<String, String> {
    let mut binding = match std::env::consts::OS {
        "windows" => std::process::Command::new("powershell"),
        _ => std::process::Command::new("bash"),
    };

    let callable = if let Some(args) = args {
        format!("{} {} {}", python.unwrap_or("python3"), path, args)
    } else {
        format!("{} {}", python.unwrap_or("python3"), path)
    };

    // println!("### {} ###", callable);

    let mut command = match std::env::consts::OS {
        "windows" => binding
            .arg("-Command")
            .arg(python.unwrap_or("python3.exe"))
            .arg(path),
        _ => binding.arg("-c").arg(callable),
    };
    if std::env::consts::OS == "windows" {
        if let Some(args) = args {
            command = command.arg(args);
        }
    }

    if let Some(envs) = envs {
        for (key, value) in envs {
            command = command.env(key, value);
        }
    }
    let output = command.output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Err(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Executes a Python script using the provided Python interpreter and returns the script's output.
///
/// # Parameters
///
/// * `script` - A reference to a string representing the Python script to be executed.
/// * `python` - An optional reference to a string representing the Python interpreter to be used.
///   If `None`, the function will default to using "python3".
///
/// # Returns
///
/// * `Result<String, String>` - On success, returns a `Result` containing the standard output of the Python script as a string.
///   On error, returns a `Result` containing the standard error of the Python script as a string.
pub fn run_python_script(script: &str, python: Option<&str>) -> Result<String, String> {
    let output = std::process::Command::new(python.unwrap_or("python3"))
        .arg("-c")
        .arg(script)
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Err(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Retrieves the platform definition by the Python interpreter.
///
/// This function executes a Python script that uses the `platform` module to determine the system and machine
/// details of the Python interpreter. The platform definition is formatted as "system-machine".
///
/// # Parameters
///
/// * `python` - An optional reference to a string representing the Python interpreter to be used.
///   If `None`, the function will default to using "python3".
///
/// # Returns
///
/// * `String` - The platform definition of the Python interpreter. If the Python script execution fails,
///   the function returns the error message as a string.
pub fn get_python_platform_definition(python: Option<&str>) -> String {
    match run_python_script(
        "import platform; print(f'{platform.system()}-{platform.machine()}')",
        python,
    ) {
        Ok(out) => out,
        Err(e) => e,
    }
}

/// Performs a series of sanity checks for the Python interpreter.
///
/// This function executes various Python scripts and checks for the availability of essential Python modules,
/// such as pip, venv, and the standard library. It also verifies the functionality of the ctypes module.
///
/// # Parameters
///
/// * `python` - An optional reference to a string representing the Python interpreter to be used.
///   If `None`, the function will default to using "python3".
///
/// # Returns
///
/// * `Vec<Result<String, String>>` - A vector of results. Each result represents the output or error message
///   of a specific Python script execution. If the script execution is successful, the result will be `Ok`
///   containing the standard output as a string. If the script execution fails, the result will be `Err`
///   containing the standard error as a string.
pub fn python_sanity_check(python: Option<&str>) -> Vec<Result<String, String>> {
    let mut outputs = Vec::new();
    // check pip
    let output = std::process::Command::new(python.unwrap_or("python3"))
        .arg("-m")
        .arg("pip")
        .arg("--version")
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                outputs.push(Ok(std::str::from_utf8(&out.stdout).unwrap().to_string()));
            } else {
                outputs.push(Err(std::str::from_utf8(&out.stderr).unwrap().to_string()));
            }
        }
        Err(e) => outputs.push(Err(e.to_string())),
    }
    // check venv
    let output_2 = std::process::Command::new(python.unwrap_or("python3"))
        .arg("-m")
        .arg("venv")
        .arg("-h")
        .output();
    match output_2 {
        Ok(out) => {
            if out.status.success() {
                outputs.push(Ok(std::str::from_utf8(&out.stdout).unwrap().to_string()));
            } else {
                outputs.push(Err(std::str::from_utf8(&out.stderr).unwrap().to_string()));
            }
        }
        Err(e) => outputs.push(Err(e.to_string())),
    }
    // check standard library
    let script = include_str!("./../python_scripts/sanity_check/import_standard_library.py");
    outputs.push(run_python_script(script, python));
    // check ctypes
    let script = include_str!("./../python_scripts/sanity_check/ctypes_check.py");
    outputs.push(run_python_script(script, python));
    // check https
    let script = include_str!("./../python_scripts/sanity_check/import_standard_library.py");
    outputs.push(run_python_script(script, python));
    outputs
}

pub fn run_python_script_with_rustpython(script: &str) -> String {
    vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let code_opbject = vm
            .compile(script, vm::compiler::Mode::Exec, "<embeded>".to_owned())
            .map_err(|err| format!("error: {:?}", err))
            .unwrap();
        let output = vm.run_code_obj(code_opbject, scope).unwrap();
        format!("output: {:?}", output)
        // Ok(output)
    });
    "".to_string()
}

pub fn py_main_idf(interp: &Interpreter) -> vm::PyResult<PyStrRef> {
    interp.enter(|vm| {
        // Add local library path
        vm.insert_sys_path(vm.new_pyobj("examples"))
            .expect("add examples to sys.path failed, why?");

        // select the idf_tools module
        let module = vm.import("idf_tools", 0)?;
        // running straight the action_install
        let name_func = module.get_attr("action_install", vm)?;
        // we will get the params from the user in the future
        let quiet = vm.ctx.false_value.clone();
        let non_interactive = vm.ctx.new_bool(false);
        let tools_json = vm.ctx.new_str("./examples/tools.json");
        let idf_path = vm.ctx.none();
        let tools = vm.ctx.new_list(vec![vm.ctx.new_str("all").into()]);
        let targets = vm.ctx.new_str("all");

        let pos_args: PosArgs = PosArgs::new(vec![
            quiet.into(),
            non_interactive.into(),
            tools_json.into(),
            idf_path,
            tools.into(),
            targets.into(),
        ]);

        let result = name_func.call(pos_args, vm)?;
        let result_str = result.str(vm)?;
        let result_pystrref: PyStrRef = result_str;
        // let result: PyStrRef = result.get_attr("name", vm)?.try_into_value(vm)?;
        vm::PyResult::Ok(result_pystrref)
    })
}

// in the future we will accept params what to actually install ;-)
pub fn run_idf_tools() -> ExitCode {
    let mut settings = vm::Settings::default();
    settings.path_list.push("Lib".to_owned()); // addng folder lib in current directory
    if let Ok(path) = env::var("RUSTPYTHONPATH") {
        settings
            .path_list
            .extend(path.split(':').map(|s| s.to_owned()));
    }
    let interp = vm::Interpreter::with_init(settings, |vm| {
        vm.add_native_modules(rustpython_stdlib::get_module_inits());
    });

    let result = py_main_idf(&interp);
    let result = result.map(|result| {
        println!("name: {result}");
    });
    ExitCode::from(interp.run(|_vm| result))
}
