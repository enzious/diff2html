use std::convert::From;

use clap::ArgMatches;

impl Default for Diff2HtmlConfig {
    fn default() -> Diff2HtmlConfig {
        Diff2HtmlConfig {
            input: "command".to_owned(),
            output: "stdout".to_owned(),
            diff: "smartword".to_owned(),
            style: "line".to_owned(),
            synchronized_scroll: "enabled".to_owned(),
            summary: "closed".to_owned(),
            matching: "none".to_owned(),
            match_words_threshold: 0.25f64,
            matching_max_comparisons: 2500,
            file: None,
            format: "html".to_owned(),
            is_combined: false,
            max_line_length_highlight: 10000,
            word_by_word: true,
            char_by_char: false,
            trail: None,
        }
    }
}

#[derive(Clone)]
pub struct Diff2HtmlConfig {
    pub input: String,
    pub output: String,
    pub diff: String,
    pub style: String,
    pub synchronized_scroll: String,
    pub summary: String,
    pub matching: String,
    pub match_words_threshold: f64,
    pub matching_max_comparisons: usize,
    pub file: Option<String>,
    pub format: String,
    pub is_combined: bool,
    pub max_line_length_highlight: usize,
    pub word_by_word: bool,
    pub char_by_char: bool,
    pub trail: Option<Vec<String>>,
}

impl<'a> From<ArgMatches<'a>> for Diff2HtmlConfig {
    fn from(matches: ArgMatches<'a>) -> Diff2HtmlConfig {
        let mut config = Diff2HtmlConfig::default();

        // input
        if let Some(input) = matches.value_of("input") {
            config.input = input.to_owned();
        }

        // file
        if let Some(file) = matches.value_of("file") {
            config.file = Some(file.to_owned());
        }

        // format
        if let Some(format) = matches.value_of("format") {
            config.format = format.to_owned();
        }

        // output
        if let Some(output) = matches.value_of("output") {
            config.output = output.to_owned();
        }

        // diff
        if let Some(diff) = matches.value_of("diff") {
            config.diff = diff.to_owned();
        }

        // style
        if let Some(style) = matches.value_of("style") {
            config.style = style.to_owned();
        }

        // synchronized_scroll
        if let Some(synchronized_scroll) = matches.value_of("synchronized_scroll") {
            config.synchronized_scroll = synchronized_scroll.to_owned();
        }

        // summary
        if let Some(summary) = matches.value_of("summary") {
            config.summary = summary.to_owned();
        }

        // matching
        if let Some(matching) = matches.value_of("matching") {
            config.matching = matching.to_owned();
        }

        // match_words_threshold
        if let Some(match_words_threshold) = matches.value_of("match_words_threshold") {
            config.match_words_threshold = match_words_threshold
                .parse()
                .expect("Match words threshold is not in float format.");
        }

        // matching_max_comparisons
        if let Some(matching_max_comparisons) = matches.value_of("matching_max_comparisons") {
            config.matching_max_comparisons = matching_max_comparisons
                .parse()
                .expect("Match words threshold is not in unsigned integer format.");
        }

        config.trail = matches
            .values_of("trail")
            .map(|v| v.map(|v| v.to_owned()).collect());

        config
    }
}
