use std::{
    env,
    fs::DirEntry,
    io::{BufRead, BufReader, Error, Write},
    process::Stdio,
    thread::sleep,
    time::Duration,
};

use anyhow::{anyhow, Result};
use args::Args;
use clap::Parser;
use lazy_static::lazy_static;

mod args;
mod macros;

const RATE_LIMIT: &str = "Rate limit exceeded";
const ALREADY_EXISTS: &str = "already exists";
const ERROR: &str = "error";
const DAY_SECS: u64 = 60 * 60 * 24;
const THIS_DIR_NAME: &str = "script";

lazy_static! {
    static ref PWD: String = env::var("PWD").unwrap_or_else(|_| ".".to_string());
}

fn main() -> Result<()> {
    let args = Args::parse();

    let folders: Vec<Result<DirEntry, Error>> = std::fs::read_dir(&*PWD)
        .map_err(|_| anyhow!("😣 获取文件夹列表失败"))?
        .collect();
    let total_count = folders.len();
    let mut counter = 0;
    for entry in folders {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                println!("😣 获取文件夹失败：{}", err);
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                println!("😣 获取文件类型失败：{}", err);
                continue;
            }
        };
        if file_type.is_file() {
            continue;
        }
        let user_name = if let Ok(name) = entry.file_name().into_string() {
            name
        } else {
            println!("😣 未能获取文件名");
            continue;
        };
        if user_name == THIS_DIR_NAME {
            continue;
        }
        // Counter pad left 3 digits
        counter += 1;
        println!(
            "[{}/{}] {}",
            format!("{:03}", counter),
            total_count,
            user_name
        );

        let img_dir = format!("{}/{}", user_name, "img");
        let vid_dir = format!("{}/{}", user_name, "video");
        if create_dir_ignore_exists(&img_dir).is_err() {
            continue;
        }
        if create_dir_ignore_exists(&vid_dir).is_err() {
            continue;
        }

        let perm_skip_flag = format!("{}/{}", user_name, ".perm_skip");
        if let Ok(meta) = std::fs::metadata(&perm_skip_flag) {
            if meta.is_file() {
                println!("💡 永久跳过");
                continue;
            }
        }

        let skip_flag = format!("{}/{}", user_name, ".skip");
        let skip_file_stat = std::fs::metadata(&skip_flag);
        match skip_file_stat {
            Ok(meta) => {
                let mod_ts = meta.modified();
                if mod_ts.is_err() {
                    println!("😣 未能获取文件修改时间");
                    continue;
                }
                let mod_ts = mod_ts.unwrap().elapsed();
                if mod_ts.is_err() {
                    println!("😣 未能获取文件修改时间");
                    continue;
                }
                let mod_ts = mod_ts.unwrap().as_secs();
                let now_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if now_ts - mod_ts < DAY_SECS {
                    println!("💡 跳过");
                    continue;
                }
            }
            _ => (),
        }

        let mut only_retweet = "";
        // 是否存在 .only_retweet 文件
        if let core::result::Result::Ok(meta) =
            std::fs::metadata(format!("{}/{}", user_name, ".retweet_only"))
        {
            if meta.is_file() {
                only_retweet = "--retweet-only";
            }
        }

        let mut cmd = std::process::Command::new("twmd");
        cmd.arg("-B");
        if !args.no_login {
            cmd.arg("--login");
        }
        cmd.arg("--all");
        cmd.arg("--update");
        cmd.arg(only_retweet);
        cmd.arg("--user");
        cmd.arg(&user_name);
        // 输出错误到标准
        cmd.stderr(std::process::Stdio::inherit());
        cmd.stdout(Stdio::piped());
        let mut child = if let Ok(child) = cmd.spawn() {
            child
        } else {
            println!("🚫 未能启动 twmd");
            continue;
        };

        if let Some(stdout) = child.stdout.take() {
            // 总行数
            let mut total_lines = match args.no_login {
                true => 0,
                false => -1,
            };
            let mut exists_lines = 0;
            let mut err_lines = 0;

            // 按行读取输出
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                total_lines += 1;
                let line = if let Ok(line) = line {
                    line
                } else {
                    continue;
                };

                // let pos = if let Ok(pos) = cursor::position() {
                //     pos
                // } else {
                //     continue;
                // };
                print_flush!("{}{}", "\x1b[0G\x1b[2K", line);

                if line.contains(format!("User '{}' not found", &user_name).as_str()) {
                    match std::fs::File::create(&perm_skip_flag) {
                        Ok(_) => println!("🚫 用户不存在，已生成 {}", &perm_skip_flag),
                        Err(err) => println!("🚫 生成永久跳过失败：{}", err),
                    }
                    break;
                }
                if line.contains(RATE_LIMIT) {
                    println!("\n🚫 {}", RATE_LIMIT);
                    sleep(Duration::from_secs(DAY_SECS));
                    break;
                }
                if line.contains(ALREADY_EXISTS) {
                    exists_lines += 1;
                    continue;
                }
                if line.contains(ERROR) {
                    err_lines += 1;
                    continue;
                }
            }

            println!(
                "\n📦 总: {}，已存在: {}，失败: {}",
                total_lines, exists_lines, err_lines
            );

            if err_lines < total_lines * 9 / 10 {
                std::fs::File::create(skip_flag).unwrap();
                println!("💡 生成 .skip 文件");
            }
        }

        // 等待子进程结束
        let _ = child.wait().expect("等待子进程失败");
        sleep(Duration::from_secs(60));
    }
    Ok(())
}

fn create_dir_ignore_exists(dir: &str) -> Result<()> {
    if let Err(err) = std::fs::create_dir(dir) {
        let err = err.to_string();
        if !err.contains("exists") {
            println!("🚫 {}", err);
            return Err(anyhow!(err));
        }
    }
    Ok(())
}
