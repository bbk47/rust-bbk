use clap::Parser;
use colored::*;
use regex::Regex;
use std::{fmt::Debug, io::Write};

use env_logger::{Builder, Env};
use log::{Level, LevelFilter};

use std::{
    fs,
    sync::{Arc, Mutex},
};

mod protocol;
// 目录模块
mod proxy;
mod serializer;
mod serve;
mod stub;
mod transport;
mod utils;
// 文件模块
mod client;
mod option;
mod server;

/// bbk is a tunnel for bypass firewall
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// config file of bbk start
    #[arg(short, long)]
    pub config: String,
}
fn main() {
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
            writeln!(buf, "{} [{}] > {}", timestamp, level, record.args())
        })
        .filter(None, LevelFilter::Info)
        .init();

    // let s = String::from("hello world");

    // let hello = &s[0..5];
    // let world = &s[6..11];
    let fscontent = match fs::read_to_string(&args.config) {
        Ok(ret) => ret,
        Err(e) => panic!("read config file  failed: {:?}", e),
    };

    let re = Regex::new(r"\x22mode\x22:\s\x22(client|server)").unwrap();
    let m = match re.find(&fscontent) {
        Some(s) => s,
        None => panic!("invalid mode in config file!"),
    };
    let retstr = m.as_str();
    let mode = &retstr[9..];
    if mode == "client" {
        let bbkopts: option::BbkCliOption = serde_json::from_str(&fscontent).unwrap();
        let jsonstr = serde_json::to_string_pretty(&bbkopts).unwrap();
        println!("bbkopts:\n{}!", jsonstr);
        // We are reusing `anstyle` but there are `anstyle-*` crates to adapt it to your
        // preferred styling crate.

        let mut cli = client::BbkClient::new(bbkopts);
        cli.bootstrap()
    } else {
        let bbkopts: option::BbkSerOption = serde_json::from_str(&fscontent).unwrap();
        let jsonstr = serde_json::to_string_pretty(&bbkopts).unwrap();
        println!("bbkopts:\n{}!", jsonstr);

        let mut svc = server::BbkServer::new(bbkopts);
        svc.bootstrap()
    }
}
