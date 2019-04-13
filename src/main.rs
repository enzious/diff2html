#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;

use std::fs;
use std::io::{ Read, Write, };

use clap::{ App, Arg, };

mod config;
mod printers;
mod parser;

use self::printers::{ file_list, LineByLinePrinter, SideBySidePrinter, };

static CSS: &'static str = include_str!("./templates/css.hbs");

fn main() {

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(Arg::with_name("config")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::with_name("style")
            .long("style")
            .value_name("STYLE")
            .possible_values(&[ "line", "side", ])
            .help("Output style")
            .takes_value(true))
        .arg(Arg::with_name("synchronisedScroll")
            .long("synchronisedScroll")
            .value_name("MODE")
            .possible_values(&[ "enabled", "disabled", ])
            .help("Synchronised horizontal scroll")
            .takes_value(true))
        .arg(Arg::with_name("summary")
            .long("summary")
            .value_name("STYLE")
            .possible_values(&[ "closed", "open", "hidden" ])
            .help("Show files summary")
            .takes_value(true))
        .arg(Arg::with_name("matching")
            .long("matching")
            .value_name("MATCHING")
            .possible_values(&[ "none", "lines", "words", "chars" ])
            .help("Diff line matching type")
            .takes_value(true))
        .arg(Arg::with_name("matchWordsThreshold")
            .long("matchWordsThreshold")
            .value_name("THRESHOLD")
            .help("Diff line matching word threshold")
            .takes_value(true))
        .arg(Arg::with_name("matchingMaxComparisons")
            .long("matchingMaxComparisons")
            .value_name("MAX")
            .help("Path to custom template to be rendered when using the html output format")
            .takes_value(true))
        .arg(Arg::with_name("format")
            .long("format")
            .value_name("FORMAT")
            .possible_values(&[ "html", "json", ])
            .help("Output format")
            .takes_value(true))
        .arg(Arg::with_name("diff")
            .long("diff")
            .value_name("STYLE")
            .help("Diff style")
            .possible_values(&[ "word", "char", ])
            .takes_value(true))
        .arg(Arg::with_name("input")
            .long("input")
            .value_name("SOURCE")
            .help("Diff input source")
            .possible_values(&[ "file", "command", "stdin", ])
            .takes_value(true))
        .arg(Arg::with_name("output")
            .long("output")
            .value_name("OUTPUT")
            .help("Output destination")
            .possible_values(&[ "preview", "stdout", ])
            .takes_value(true))
        .arg(Arg::with_name("file")
            .long("file")
            .value_name("FILE")
            .help("Send output to file (overrides output option)")
            .takes_value(true))
        .arg(Arg::with_name("ignore")
            .long("ignore")
            .value_name("FILES")
            .help("Ignore particular files from the diff")
            .takes_value(true)
            .multiple(true))
        .get_matches();

    let config: config::Diff2HtmlConfig = matches.into();

    // test_diffs/fuck.diff, test_diffs/test.diff, test_diffs/linux-5.1-rc4.diff
    let mut input = Vec::new();
    if config.input == "stdin" {
        ::std::io::stdin().read_to_end(&mut input).unwrap();
    } else if let Some(file) = config.file.to_owned() {
        let mut file = fs::File::open(file).unwrap();
        file.read_to_end(&mut input).unwrap();
    } else {
        eprintln!("No file was supplied.");
    }

    let diff_raw = String::from_utf8_lossy(&input);

    let files = parser::DiffParser::new().parse_diff(&diff_raw);

    let summary = if config.summary != "hidden" {
        file_list::generate_file_list_summary(&files)
    } else {
        "".to_owned()
    };

    let content = if config.style == "line" {
        LineByLinePrinter::new(config.to_owned()).render(&files)
    } else {
        SideBySidePrinter::new(config.to_owned()).render(&files)
    };

    let html_out = format!(r#"
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <style type="text/css">
                    body
                    {{
                        font-family: Roboto,sans-serif;
                        font-size: 16px;
                        line-height: 1.6;
                    }}
                    *
                    {{
                        -webkit-box-sizing: border-box;
                        -moz-box-sizing: border-box;
                        box-sizing: border-box;
                    }}
                    table
                    {{
                        background: white;
                    }}
                    {}
                </style>
            </head>
            <body>
                {}
                {}
            </body>
        </html>
    "#, CSS, &summary, &content);

    let mut out = match &config.output as &str {
        "stdout" => std::io::stdout(),
        _ => {
            panic!("Invalid output type.");
        },
    };

    writeln!(&mut out, "{}", &html_out).expect("Failed to write out.");

}