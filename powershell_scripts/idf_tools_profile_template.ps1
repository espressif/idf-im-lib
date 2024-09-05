# Set environment variables
$env:IDF_PATH = '{{idf_path}}'
$env:IDF_TOOLS_PATH = '{{idf_tools_path}}'
$env:IDF_PYTHON_ENV_PATH = '{{idf_tools_path}}\python\'

# Set system path
$env:PATH += ";{{add_paths_extras}}"

# Define the Invoke-idfpy function
function global:Invoke-idfpy {
    {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\tools\idf.py" @args
}

function global:esptool.py {
  {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\components\esptool_py\esptool\esptool.py" @args
}

function global:espefuse.py {
  {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\components\esptool_py\esptool\espefuse.py" @args
}

function global:espsecure.py {
  {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\components\esptool_py\esptool\espsecure.py" @args
}

function global:otatool.py {
  {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\components\app_update\otatool.py" @args
}

function global:parttool.py {
  {{idf_tools_path}}\python\Scripts\python.exe "{{idf_path}}\components\partition_table\parttool.py" @args
}

# Create an alias for the function
New-Alias -Name idf.py -Value Invoke-idfpy -Force -Scope Global

# Activate your Python environment
. '{{idf_tools_path}}\python\Scripts\Activate.ps1'

# Display setup information
Write-Host 'IDF PowerShell Environment' -ForegroundColor Green
Write-Host '-------------------------' -ForegroundColor Green
Write-Host 'Environment variables set:' -ForegroundColor Cyan
Write-Host "IDF_PATH: $env:IDF_PATH" -ForegroundColor Yellow
Write-Host "IDF_TOOLS_PATH: $env:IDF_TOOLS_PATH" -ForegroundColor Yellow
Write-Host "IDF_PYTHON_ENV_PATH: $env:IDF_PYTHON_ENV_PATH" -ForegroundColor Yellow
Write-Host ''
Write-Host 'Custom commands available:' -ForegroundColor Cyan
Write-Host 'idf.py - Use this to run IDF commands (e.g., idf.py build)' -ForegroundColor Yellow
Write-Host 'esptool.py' -ForegroundColor Yellow
Write-Host 'espefuse.py' -ForegroundColor Yellow
Write-Host 'espsecure.py' -ForegroundColor Yellow
Write-Host 'otatool.py' -ForegroundColor Yellow
Write-Host 'parttool.py' -ForegroundColor Yellow
Write-Host ''
Write-Host 'Python environment activated.' -ForegroundColor Cyan
Write-Host 'You can now use IDF commands and Python tools.' -ForegroundColor Cyan