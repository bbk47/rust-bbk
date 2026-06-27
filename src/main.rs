use clap::Parser;
use colored::*;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use env_logger::Builder;
use log::{Level, LevelFilter};

use bbk::{client, option, server};

/// bbk is a tunnel for bypassing firewalls.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// config file of bbk start
    #[arg(short, long)]
    pub config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.config.is_empty() {
        println!("config file is missing!");
        return;
    }

    Builder::new()
        .format(|buf, record| {
            let timestamp = buf.timestamp();
            let level = match record.level() {
                Level::Error => record.level().to_string().red(),
                Level::Warn => record.level().to_string().yellow(),
                Level::Info => record.level().to_string().green(),
                Level::Debug | Level::Trace => record.level().to_string().blue(),
            };
            writeln!(buf, "{} {} > {}", timestamp, level, record.args())
        })
        .filter(None, LevelFilter::Info)
        .init();

    let fscontent = match fs::read_to_string(&args.config) {
        Ok(ret) => ret,
        Err(e) => panic!("read config file failed: {:?}", e),
    };

    let re = Regex::new(r#""mode":\s*"(client|server)""#).unwrap();
    let caps = match re.captures(&fscontent) {
        Some(c) => c,
        None => panic!("invalid mode in config file!"),
    };
    let mode = &caps[1];

    if mode == "client" {
        let bbkopts: option::BbkCliOption = serde_json::from_str(&fscontent).unwrap();
        let jsonstr = serde_json::to_string_pretty(&bbkopts).unwrap();
        println!("bbkopts:\n{}", jsonstr);

        let cli = client::BbkClient::new(bbkopts);
        cli.bootstrap().await;
    } else {
        let mut bbkopts: option::BbkSerOption = serde_json::from_str(&fscontent).unwrap();
        let jsonstr = serde_json::to_string_pretty(&bbkopts).unwrap();
        println!("bbkopts:\n{}", jsonstr);

        if !bbkopts.ssl_crt.is_empty() {
            bbkopts.ssl_crt = readfile_as_str(&bbkopts.ssl_crt);
        }
        if !bbkopts.ssl_key.is_empty() {
            bbkopts.ssl_key = readfile_as_str(&bbkopts.ssl_key);
        }
        let svc = server::BbkServer::new(bbkopts);
        svc.bootstrap().await;
    }
}

fn readfile_as_str(filepath: &str) -> String {
    let mut file = File::open(Path::new(filepath)).expect("cannot open file");
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).expect("read file failed");
    buffer
}
