#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::path::{Path, PathBuf};

fn get_cli_path() -> PathBuf {
    if let Ok(env_path) = std::env::var("CH_CLI_PATH") {
        return PathBuf::from(env_path);
    }
    
    let home = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(PathBuf::from))
        .unwrap_or_else(|_| PathBuf::from("/"));
        
    // Check Desktop/chunhui-cil/ch_cli.py
    let mut path = home.clone();
    path.push("Desktop");
    path.push("chunhui-cil");
    path.push("ch_cli.py");
    if path.exists() {
        return path;
    }
    
    // Fallback if not exists
    path
}

fn run_python_command(cli_path: &Path, args: &[String]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("python3");
    cmd.arg(cli_path).args(args);
    
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(_) => {
            // Try fallback to "python"
            let mut cmd_fallback = std::process::Command::new("python");
            cmd_fallback.arg(cli_path).args(args);
            match cmd_fallback.output() {
                Ok(output) => {
                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        Err(String::from_utf8_lossy(&output.stderr).to_string())
                    }
                }
                Err(e) => Err(format!("无法执行 Python 解释器 (python3/python): {}", e)),
            }
        }
    }
}

#[tauri::command]
fn run_cli(args: Vec<String>) -> Result<String, String> {
    let cli_path = get_cli_path();
    if !cli_path.exists() {
        return Err(format!("找不到 CLI 脚本文件: {:?}", cli_path));
    }
    run_python_command(&cli_path, &args)
}

#[tauri::command]
fn run_bridge(cmd: String, args: Vec<String>) -> Result<String, String> {
    let mut bridge_path = get_cli_path();
    bridge_path.set_file_name("cwu_bridge.py");
    if !bridge_path.exists() {
        return Err(format!("找不到 Bridge 脚本文件: {:?}", bridge_path));
    }
    
    let mut run_args = vec![bridge_path.to_string_lossy().to_string(), cmd];
    run_args.extend(args);
    
    let mut child = std::process::Command::new("python3");
    child.args(&run_args);
    
    match child.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                Ok(stdout)
            } else {
                Err(format!("执行 Bridge 错误: {}\n{}", stderr, stdout))
            }
        }
        Err(_) => {
            let mut child_fallback = std::process::Command::new("python");
            child_fallback.args(&run_args);
            match child_fallback.output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    if output.status.success() {
                        Ok(stdout)
                    } else {
                        Err(format!("执行 Bridge 错误: {}\n{}", stderr, stdout))
                    }
                }
                Err(e) => Err(format!("无法执行 Python (python3/python): {}", e)),
            }
        }
    }
}

#[tauri::command]
fn open_tui() -> Result<(), String> {
    let cli_path = get_cli_path();
    if !cli_path.exists() {
        return Err(format!("找不到 CLI 脚本文件: {:?}", cli_path));
    }
    
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("cmd")
            .args(&["/c", "start", "python", &cli_path.to_string_lossy()])
            .status();
        
        match status {
            Ok(s) if s.success() => return Ok(()),
            _ => {
                let status3 = std::process::Command::new("cmd")
                    .args(&["/c", "start", "python3", &cli_path.to_string_lossy()])
                    .status();
                match status3 {
                    Ok(s) if s.success() => return Ok(()),
                    Ok(_) => return Err("命令行进程启动失败".to_string()),
                    Err(e) => return Err(format!("启动失败: {}", e)),
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(&["-a", "Terminal", &cli_path.to_string_lossy()])
            .status()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        return Err("不支持的操作系统".to_string());
    }
    
    Ok(())
}

#[tauri::command]
fn set_env() -> Result<String, String> {
    let cli_path = get_cli_path();
    
    #[cfg(target_os = "macos")]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        
        let home = std::env::var("HOME").map_err(|e| e.to_string())?;
        let zshrc_path = format!("{}/.zshrc", home);
        
        let alias_line = format!("\nalias chunhui='python3 {}'\n", cli_path.to_string_lossy());
        
        let mut file = OpenOptions::new()
            .append(true)
            .open(&zshrc_path)
            .map_err(|e| format!("无法打开 .zshrc: {}", e))?;
            
        file.write_all(alias_line.as_bytes())
            .map_err(|e| format!("无法写入 .zshrc: {}", e))?;
            
        Ok("成功向 ~/.zshrc 追加别名配置，请重启终端执行 'source ~/.zshrc' 生效。".to_string())
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        let home = std::env::var("USERPROFILE").map_err(|e| e.to_string())?;
        let bat_path = format!("{}\\chunhui.bat", home);
        
        let mut file = std::fs::File::create(&bat_path)
            .map_err(|e| format!("无法创建 bat 文件: {}", e))?;
            
        let content = format!("@echo off\npython \"{}\" %*\n", cli_path.to_string_lossy());
        file.write_all(content.as_bytes())
            .map_err(|e| format!("无法写入 bat 文件: {}", e))?;
            
        Ok(format!("成功在用户目录创建 {}。请将用户目录加入 PATH 环境变量以直接在终端输入 'chunhui' 调用命令行客户端。", bat_path))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("不支持的操作系统".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_cli, run_bridge, open_tui, set_env])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
