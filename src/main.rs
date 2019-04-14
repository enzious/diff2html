#![feature(pattern)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;

use std::fs;
use std::io::{Read, Write};
use std::process::Command;

use clap::{App, AppSettings, Arg, ArgMatches};

mod config;
mod difference;
mod parse;
mod printers;

use crate::config::Diff2HtmlConfig;
use crate::printers::PagePrinter;

fn main() {
    let config: Diff2HtmlConfig = get_arg_matches().into();
    let input = get_input(&config);
    let output = get_output(&config, &input);
    handle_output(&config, &output);
}

fn get_input(config: &Diff2HtmlConfig) -> String {
    let mut input = Vec::new();
    if config.input == "stdin" {
        ::std::io::stdin().read_to_end(&mut input).unwrap();
    } else if config.input == "file" {
        let trailing = config.trail.as_ref().expect("No input file specified.");
        let file_name = trailing.get(0).expect("No input file specified.");
        let mut file = fs::File::open(file_name).unwrap();
        file.read_to_end(&mut input).unwrap();
    } else {
        input = get_git_diff(config);
    }
    String::from_utf8_lossy(&input[..]).to_string()
}

fn get_output(config: &Diff2HtmlConfig, input: &str) -> String {
    let mut config = config.to_owned();
    config.word_by_word = config.diff == "word";
    config.char_by_char = config.diff == "char" || config.diff == "smartword";

    let files = parse::parse_diff(&input);

    if config.format == "html" {
        let page_printer = PagePrinter::new(config);
        page_printer.render(&files)
    } else {
        serde_json::to_string(&files).unwrap()
    }
}

fn handle_output(config: &Diff2HtmlConfig, output: &str) {
    let mut out: Box<Write> = if let Some(file) = &config.file {
        Box::new(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(file)
                .unwrap(),
        )
    } else {
        match &config.output as &str {
            "stdout" => Box::new(std::io::stdout()),
            _ => {
                panic!("Invalid output type.");
            }
        }
    };
    writeln!(&mut out, "{}", &output).expect("Failed to write out.");
}

fn get_arg_matches() -> ArgMatches<'static> {
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name("config")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("style")
                .long("style")
                .value_name("STYLE")
                .possible_values(&["line", "side"])
                .help("Output style")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("synchronisedScroll")
                .long("synchronisedScroll")
                .value_name("MODE")
                .possible_values(&["enabled", "disabled"])
                .help("Synchronised horizontal scroll")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("summary")
                .long("summary")
                .value_name("STYLE")
                .possible_values(&["closed", "open", "hidden"])
                .help("Show files summary")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("matching")
                .long("matching")
                .value_name("MATCHING")
                .possible_values(&["none", "lines", "words", "smartword"])
                .help("Diff line matching type")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("matchWordsThreshold")
                .long("matchWordsThreshold")
                .value_name("THRESHOLD")
                .help("Diff line matching word threshold")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("matchingMaxComparisons")
                .long("matchingMaxComparisons")
                .value_name("MAX")
                .help("Path to custom template to be rendered when using the html output format")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("format")
                .long("format")
                .value_name("FORMAT")
                .possible_values(&["html", "json"])
                .help("Output format")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("diff")
                .long("diff")
                .value_name("STYLE")
                .help("Diff style")
                .possible_values(&["word", "char"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("input")
                .long("input")
                .value_name("SOURCE")
                .help("Diff input source")
                .possible_values(&["file", "command", "stdin"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .value_name("OUTPUT")
                .help("Output destination")
                .possible_values(&["preview", "stdout"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("file")
                .long("file")
                .value_name("FILE")
                .help("Send output to file (overrides output option)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ignore")
                .long("ignore")
                .value_name("FILES")
                .help("Ignore particular files from the diff")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("trail")
                .value_name("FILE/ARGS")
                .multiple(true),
        )
        .get_matches()
}

fn get_git_diff(config: &Diff2HtmlConfig) -> Vec<u8> {
    let mut args: Vec<String> = match &config.trail {
        Some(trailing) => trailing.to_owned(),
        _ => vec!["-M", "-C", "HEAD"]
            .iter()
            .map(|v| v.to_string())
            .collect(),
    };

    if !args.contains(&"--no-color".to_owned()) {
        args.push("--no-color".to_owned());
    }

    Command::new("git")
        .arg("--no-pager")
        .arg("diff")
        .args(args)
        .output()
        .unwrap()
        .stdout
}
