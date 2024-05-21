use std::{
    env,
    fs::{self, DirEntry, File},
    io::{BufRead, BufReader, Error, Write},
    process::{self, Stdio},
    thread::sleep,
    time::Duration,
};

use anyhow::{anyhow, Result};
use args::Args;
use clap::Parser;
use utils::*;

mod args;
mod macros;
mod utils;

const RATE_LIMIT: &str = "Rate limit exceeded";
const ALREADY_EXISTS: &str = "already exists";
const DOWNLOADED: &str = "Downloaded";
const ERROR: &str = "error";
const DAY_SECS: u64 = 60 * 60 * 24;

fn main() -> Result<()> {
    let args = Args::parse();

    let pwd = env::var("PWD").unwrap_or_else(|_| ".".to_string());

    let folders: Vec<Result<DirEntry, Error>> = fs::read_dir(&pwd)
        .map_err(|_| anyhow!("😣 获取文件夹列表失败"))?
        .collect();
    let total_count = folders.len();
    println!("🚀 开始，总计 {} 个", total_count);

    let mut counter = 0;
    let mut perm_skip_names = Vec::new();
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
        if args.skip_dirs.contains(&user_name) {
            println!("\n👋🏻 自定义跳过 {}", &user_name);
            continue;
        }

        counter += 1;
        println!("\n[{}/{}] {}", counter, total_count, &user_name);

        let img_dir = format!("{}/{}", &user_name, "img");
        let vid_dir = format!("{}/{}", &user_name, "video");
        if create_dir_ignore_exists(&img_dir).is_err() {
            continue;
        }
        if create_dir_ignore_exists(&vid_dir).is_err() {
            continue;
        }

        let perm_skip_flag = format!("{}/{}", &user_name, ".perm_skip");
        if let Ok(meta) = fs::metadata(&perm_skip_flag) {
            if meta.is_file() {
                println!("💡 永久跳过");
                continue;
            }
        }

        let skip_flag = format!("{}/{}", &user_name, ".skip");
        let skip_file_stat = fs::metadata(&skip_flag);
        match skip_file_stat {
            Ok(meta) => {
                let mod_ts = if let Ok(mod_ts) = meta.modified() {
                    mod_ts
                } else {
                    println!("😣 未能获取文件修改时间");
                    continue;
                };
                let mod_ts = mod_ts.elapsed().unwrap_or_default().as_secs();
                if mod_ts < DAY_SECS {
                    println!("🕒 上次更新在 {} 前，跳过", secs_to_human(mod_ts));
                    continue;
                }
            }
            _ => (),
        }

        let mut only_retweet = "";
        // 是否存在 .only_retweet 文件
        if let Ok(meta) = fs::metadata(format!("{}/{}", &user_name, ".retweet_only")) {
            if meta.is_file() {
                only_retweet = "--retweet-only";
            }
        }

        let mut cmd = process::Command::new("twmd");
        cmd.arg("-B");
        if !args.no_login {
            cmd.arg("--login");
        }
        cmd.arg("--all");
        cmd.arg("--update");
        cmd.arg(only_retweet);
        cmd.arg("--user");
        cmd.arg(&user_name);
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
            let mut dl_lines = 0;
            // 不正常的行（非 already exists 和 Download）
            let mut abnormal_lines = Vec::new();

            // 按行读取输出
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                total_lines += 1;
                let line = if let Ok(line) = line {
                    line
                } else {
                    continue;
                };

                print_flush!("{}{}", "\x1b[0G\x1b[2K", line);

                if line.contains(format!("User '{}' not found", &user_name).as_str()) {
                    perm_skip_names.push(user_name.clone());
                    match File::create(&perm_skip_flag) {
                        Ok(_) => println!("\n🚫 用户不存在，已生成 {}", &perm_skip_flag),
                        Err(err) => println!("\n🚫 生成永久跳过失败：{}", err),
                    }
                    if child.kill().is_ok() {
                        println!("🚫 已终止");
                    }
                    break;
                }
                if line.contains(RATE_LIMIT) {
                    println!("\n🚫 {}", RATE_LIMIT);
                    if child.kill().is_ok() {
                        println!("🚫 已终止");
                    }
                    sleep(Duration::from_secs(args.sleep * 30));
                    break;
                }
                if line.contains(ALREADY_EXISTS) {
                    exists_lines += 1;
                    continue;
                }
                if line == ERROR {
                    err_lines += 1;
                    continue;
                }
                if line.starts_with(DOWNLOADED) {
                    dl_lines += 1;
                    continue;
                }
                abnormal_lines.push(line);
            }

            println!(
                "\n📦 总 {}，下载 {}，存在 {}，失败 {}",
                total_lines, dl_lines, exists_lines, err_lines
            );

            if err_lines <= total_lines / 10 {
                // 如果无法生成 .skip 文件，直接 panic，设计如此
                File::create(skip_flag).unwrap();
                println!("💡 生成 .skip 文件");
            }

            if !abnormal_lines.is_empty() {
                let path = format!("{}/{}", &user_name, ".abnormal");
                fs::write(&path, abnormal_lines.join("\n"))?;
                println!("🚫 异常行已写入 {}", &path);
            }
        }

        let _ = child.wait()?;
        sleep(Duration::from_secs(args.sleep));
    }

    if !perm_skip_names.is_empty() {
        println!("🚫 永久跳过：{:?}", perm_skip_names);
    }
    println!("🎉 完成");

    Ok(())
}
