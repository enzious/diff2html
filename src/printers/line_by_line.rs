use std::cmp::min;

use handlebars::Handlebars;
use v_htmlescape::escape;

use crate::{ parser, config::Diff2HtmlConfig, };
use super::utils::{ self, rematch, Difference, };

static GENERIC_COLUMN_LINE_NUMBER: &'static str = include_str!("../templates/generic-column-line-number.hbs");
static GENERIC_EMPTY_DIFF: &'static str = include_str!("../templates/generic-empty-diff.hbs");
static GENERIC_FILE_PATH: &'static str = include_str!("../templates/generic-file-path.hbs");
static GENERIC_LINE: &'static str = include_str!("../templates/generic-line.hbs");
static GENERIC_WRAPPER: &'static str = include_str!("../templates/generic-wrapper.hbs");
static LINE_BY_LINE_FILE_DIFF: &'static str = include_str!("../templates/line-by-line-file-diff.hbs");
static LINE_BY_LINE_NUMBERS: &'static str = include_str!("../templates/line-by-line-numbers.hbs");
static ICON_FILE: &'static str = include_str!("../templates/icon-file.hbs");

pub struct LineByLinePrinter {
    config: Diff2HtmlConfig,
    handlebars: Handlebars,
    line_matcher: rematch::Rematcher<parser::Line>,
    diff_matcher: rematch::Rematcher<Difference>,
}

impl LineByLinePrinter {

    pub fn new(config: Diff2HtmlConfig) -> LineByLinePrinter {

        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("generic-column-line-number", GENERIC_COLUMN_LINE_NUMBER).unwrap();
        handlebars.register_template_string("generic-empty-diff", GENERIC_EMPTY_DIFF).unwrap();
        handlebars.register_template_string("generic-file-path", GENERIC_FILE_PATH).unwrap();
        handlebars.register_template_string("generic-line", GENERIC_LINE).unwrap();
        handlebars.register_template_string("generic-wrapper", GENERIC_WRAPPER).unwrap();
        handlebars.register_template_string("line-by-line-numbers", LINE_BY_LINE_NUMBERS).unwrap();
        handlebars.register_template_string("line-by-line-file-diff", LINE_BY_LINE_FILE_DIFF).unwrap();

        LineByLinePrinter {
            config: config,
            handlebars: handlebars,
            line_matcher: utils::get_line_matcher(),
            diff_matcher: utils::get_difference_matcher(),
        }
    }

    pub fn render(&self, files: &Vec<parser::File>) -> String {

        let output = files.iter().map(|file| {
            let diffs = if file.blocks.len() > 0 {
                self.generate_file_html(file)
            } else {
                utils::generate_empty_diff(&self.handlebars, "d2h-code-side-line")
            };
            self.generate_file_diff_html(file, diffs)
        }).collect::<Vec<String>>().join("\n");

        self.handlebars.render("generic-wrapper", &json!({
            "content": output,
        })).unwrap()

    }

    fn generate_file_diff_html(&self, file: &parser::File, diffs: String) -> String {
        let file_path = self.handlebars.render("generic-file-path", &json!({
            "fileDiffName": utils::get_diff_name(file),
            "fileIcon": ICON_FILE,
            "fileTag": utils::get_line_type_tag(file).to_owned(),
        })).unwrap();
        self.handlebars.render("line-by-line-file-diff", &json!({
            "file": file.to_owned(),
            "fileHtmlId": utils::get_html_id(file),
            "diffs": diffs,
            "filePath": file_path,
        })).unwrap()
    }

    fn generate_file_html(&self, file: &parser::File) -> String {
        file.blocks.iter().map(|block| {

            let mut lines = utils::make_column_line_number_html(
                &self.handlebars, block.header.as_ref().unwrap(),
                "d2h-code-linenumber", "d2h-code-line",
            );

            let mut old_lines = Vec::new();
            let mut new_lines = Vec::new();

            for i in 0..block.lines.len() {

                let line = &block.lines[i];
                let escaped_line = escape(&line.content).to_string();

                if
                    line.line_type != Some(parser::LineType::Inserts)
                    && (
                        new_lines.len() > 0
                        || (
                            line.line_type != Some(parser::LineType::Deletes)
                            && old_lines.len() > 0
                        )
                    )
                {
                    self.process_change_block(file, &mut lines, &mut old_lines, &mut new_lines);
                }

                if line.line_type == Some(parser::LineType::Context) {
                    lines += &self.generate_line_html(
                        file.is_combined, line.line_type.as_ref().unwrap(),
                        line.old_number, line.new_number, escaped_line, None,
                    );
                } else if line.line_type == Some(parser::LineType::Inserts) && old_lines.len() == 0 {
                    lines += &self.generate_line_html(
                        file.is_combined, line.line_type.as_ref().unwrap(),
                        line.old_number, line.new_number, escaped_line, None,
                    );
                } else if line.line_type == Some(parser::LineType::Deletes) {
                    old_lines.push(line.to_owned());
                } else if line.line_type == Some(parser::LineType::Inserts) && old_lines.len() > 0 {
                    new_lines.push(line.to_owned());
                } else {
                    eprintln!("Unknown state in html line-by-line-generator.");
                    self.process_change_block(file, &mut lines, &mut old_lines, &mut new_lines);
                }
            }

            self.process_change_block(file, &mut lines, &mut old_lines, &mut new_lines);

            lines

        }).collect::<Vec<String>>().join("\n")
    }

    fn process_change_block(
        &self,
        file: &parser::File,
        lines: &mut String,
        old_lines: &mut Vec<parser::Line>,
        new_lines: &mut Vec<parser::Line>
    ) {

        let comparisons = old_lines.len() * new_lines.len();
        let max_comparisons = 2500;
        let do_matching = comparisons < max_comparisons && (
            self.config.matching != "none"
        );

        let old_lines2 = old_lines.to_owned();
        let new_lines2 = new_lines.to_owned();
        let ( matches, insert_type, delete_type, ) = {
            if do_matching {
                (
                    self.line_matcher.matches(&old_lines2, &new_lines2),
                    parser::LineType::InsertChanges,
                    parser::LineType::DeleteChanges,
                )
            } else {
                (
                    vec![vec![ old_lines2.as_ref(), new_lines2.as_ref(), ]],
                    parser::LineType::Inserts,
                    parser::LineType::Deletes,
                )
            }
        };

        matches.iter().for_each(|item| {

            *old_lines = item[0].to_vec();
            *new_lines = item[1].to_vec();

            let mut processed_old_lines = String::new();
            let mut processed_new_lines = String::new();

            let common = min(old_lines.len(), new_lines.len());

            let mut j = 0;
            let mut old_line;
            let mut new_line;
            while j < common {
                old_line = Some(&old_lines[j]);
                new_line = Some(&new_lines[j]);

                // TODO: hmmm
                //self.is_combined = file.is_combined;

                let diff = utils::diff_highlight(
                    &self.config,
                    Some(&self.diff_matcher),
                    &old_line.as_ref().unwrap().content,
                    &new_line.as_ref().unwrap().content,
                );

                processed_old_lines +=
                    &self.generate_line_html(
                        file.is_combined, &delete_type,
                        old_line.as_ref().and_then(|v| v.old_number),
                        old_line.as_ref().and_then(|v| v.new_number),
                        diff.first.line, Some(diff.first.prefix),
                    );
                processed_new_lines +=
                    &self.generate_line_html(
                        file.is_combined, &insert_type,
                        new_line.as_ref().and_then(|v| v.old_number),
                        new_line.as_ref().and_then(|v| v.new_number),
                        diff.second.line, Some(diff.second.prefix),
                    );

                j += 1;
            }

            *lines += &processed_old_lines as &str;
            *lines += &processed_new_lines as &str;

            *lines += &self.process_lines(file.is_combined, &old_lines[common..], &new_lines[common..]);
        });

        *old_lines = Vec::new();
        *new_lines = Vec::new();

    }

    fn generate_line_html(
        &self,
        is_combined: bool,
        line_type: &parser::LineType,
        old_number: Option<usize>,
        new_number: Option<usize>,
        content: String,
        possible_prefix: Option<&str>,
    ) -> String {

        let line_number = self.handlebars.render("line-by-line-numbers", &json!({
            "oldNumber": old_number,
            "newNumber": new_number,
        })).unwrap();

        let ( prefix, line_without_prefix, ) = match possible_prefix {
            Some(prefix) => ( prefix, content, ),
            _ => {
                let line_with_prefix = utils::separate_prefix(is_combined, &content);
                ( line_with_prefix.prefix, line_with_prefix.line.to_owned(), )
            }
        };

        self.handlebars.render("generic-line", &json!({
            "type": utils::get_line_type_class(line_type).to_owned(),
            "lineClass": "d2h-code-linenumber".to_owned(),
            "contentClass": "d2h-code-line".to_owned(),
            "prefix": prefix.to_owned(),
            "content": line_without_prefix,
            "lineNumber": line_number,
        })).unwrap()
    }

    fn process_lines(&self, is_combined: bool, old_lines: &[parser::Line], new_lines: &[parser::Line]) -> String {
        let mut lines = String::new();

        for i in 0..old_lines.len() {
            let old_line = &old_lines[i];
            let old_escaped_line = escape(&old_line.content);
            lines +=
                &self.generate_line_html(
                    is_combined, old_line.line_type.as_ref().unwrap(),
                    old_line.old_number,
                    old_line.new_number,
                    old_escaped_line.to_string(), None,
                );
        }

        for j in 0..new_lines.len() {
            let new_line = &new_lines[j];
            let new_escaped_line = escape(&new_line.content);
            lines +=
                &self.generate_line_html(
                    is_combined, new_line.line_type.as_ref().unwrap(),
                    new_line.old_number,
                    new_line.new_number,
                    new_escaped_line.to_string(), None,
                );
        }

        lines
    }

}