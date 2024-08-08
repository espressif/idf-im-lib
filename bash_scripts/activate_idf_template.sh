#!/bin/bash

# Function to add an environment variable
add_env_variable() {
    export IDF_PATH = '{{idf_path}}'
    echo "Added environment variable IDF_PATH = $IDF_PATH"
    export IDF_TOOLS_PATH = '{{idf_tools_path}}'
    echo "Added environment variable IDF_TOOLS_PATH = $IDF_TOOLS_PATH"
    export IDF_PYTHON_ENV_PATH = '{{idf_tools_path}}\python\'
    echo "Added environment variable IDF_PYTHON_ENV_PATH = $IDF_PYTHON_ENV_PATH"

}

# Function to add a directory to the system PATH
add_to_path() {
    export PATH="$PATH:{{addition_to_path}}"
    echo "Added {{addition_to_path}} to PATH"
}

# Function to activate a Python virtual environment
activate_venv() {
    VENV_PATH="$1"
    if [ -f "$VENV_PATH/bin/activate" ]; then
        source "$VENV_PATH/bin/activate"
        echo "Activated virtual environment at $VENV_PATH"
    else
        echo "Virtual environment not found at $VENV_PATH"
        return 1
    fi
}

# Main execution
add_env_variable
add_to_path

# Activate virtual environment (uncomment and provide the correct path)
activate_venv $IDF_PYTHON_ENV_PATH

echo "Environment setup complete for the current shell session."
echo "These changes will be lost when you close this terminal."
echo "You are now using IDF version {{idf_version}}."