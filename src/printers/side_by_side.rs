use std::cmp::{ max, min, };

use handlebars::Handlebars;
use v_htmlescape::escape;

use crate::{ parser, config::Diff2HtmlConfig, };
use super::utils::{ self, rematch, Difference, };

static GENERIC_COLUMN_LINE_NUMBER: &'static str = include_str!("../templates/generic-column-line-number.hbs");
static GENERIC_EMPTY_DIFF: &'static str = include_str!("../templates/generic-empty-diff.hbs");
static GENERIC_FILE_PATH: &'static str = include_str!("../templates/generic-file-path.hbs");
static GENERIC_LINE: &'static str = include_str!("../templates/generic-line.hbs");
static GENERIC_WRAPPER: &'static str = include_str!("../templates/generic-wrapper.hbs");
static SIDE_BY_SIDE_FILE_DIFF: &'static str = include_str!("../templates/side-by-side-file-diff.hbs");
static ICON_FILE: &'static str = include_str!("../templates/icon-file.hbs");

pub struct SideBySidePrinter {
    config: Diff2HtmlConfig,
    handlebars: Handlebars,
    line_matcher: rematch::Rematcher<parser::Line>,
    diff_matcher: rematch::Rematcher<Difference>,
}

impl SideBySidePrinter {

    pub fn new(config: Diff2HtmlConfig) -> SideBySidePrinter {

        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("generic-column-line-number", GENERIC_COLUMN_LINE_NUMBER).unwrap();
        handlebars.register_template_string("generic-empty-diff", GENERIC_EMPTY_DIFF).unwrap();
        handlebars.register_template_string("generic-file-path", GENERIC_FILE_PATH).unwrap();
        handlebars.register_template_string("generic-line", GENERIC_LINE).unwrap();
        handlebars.register_template_string("generic-wrapper", GENERIC_WRAPPER).unwrap();
        handlebars.register_template_string("side-by-side-file-diff", SIDE_BY_SIDE_FILE_DIFF).unwrap();

        SideBySidePrinter {
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
                self.generate_empty_diff()
            };
            self.make_file_diff_html(file, diffs)
        }).collect::<Vec<String>>().join("\n");

        self.handlebars.render("generic-wrapper", &json!({
            "content": output,
        })).unwrap()

    }

    fn make_file_diff_html(&self, file: &parser::File, diffs: SideBySideFile) -> String {

        let file_path = self.handlebars.render("generic-file-path", &json!({
            "fileDiffName": utils::get_diff_name(file),
            "fileIcon": ICON_FILE,
            "fileTag": utils::get_line_type_tag(file).to_owned(),
        })).unwrap();

        self.handlebars.render("side-by-side-file-diff", &json!({
            "file": file.to_owned(),
            "fileHtmlId": utils::get_html_id(file),
            "diffs": diffs,
            "filePath": file_path,
        })).unwrap()
    }

    fn generate_file_html(&self, file: &parser::File) -> SideBySideFile {

        let mut file_html = SideBySideFile::new();

        file.blocks.iter().for_each(|block| {

            file_html.left += &utils::make_column_line_number_html(
                &self.handlebars, block.header.as_ref().unwrap(),
                "d2h-code-side-linenumber", "d2h-code-side-line",
            );
            file_html.right += &utils::make_column_line_number_html(
                &self.handlebars, "",
                "d2h-code-side-linenumber", "d2h-code-side-line",
            );

            let mut old_lines = Vec::new();
            let mut new_lines = Vec::new();

            for i in 0..block.lines.len() {
                let line = &block.lines[i];
                let prefix = &line.content[0..1];
                let escaped_line = escape(&line.content[1..]).to_string();

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
                    self.process_change_block(file, &mut file_html, &mut old_lines, &mut new_lines);
                }

                if line.line_type == Some(parser::LineType::Context) {
                    file_html.left += &self.generate_line_html(
                        file.is_combined, &line.line_type.as_ref().unwrap(),
                        line.old_number,
                        escaped_line.to_owned(), Some(prefix),
                    );
                    file_html.right += &self.generate_line_html(
                        file.is_combined, &line.line_type.as_ref().unwrap(),
                        line.new_number,
                        escaped_line.to_owned(), Some(prefix),
                    );
                } else if line.line_type == Some(parser::LineType::Inserts) && old_lines.len() == 0 {
                    file_html.left += &self.generate_line_html(
                        file.is_combined, &parser::LineType::Context,
                        None, "".to_owned(), None,
                    );
                    file_html.right += &self.generate_line_html(
                        file.is_combined, &line.line_type.as_ref().unwrap(),
                        line.new_number,
                        escaped_line.to_owned(), Some(prefix),
                    );
                } else if line.line_type == Some(parser::LineType::Deletes) {
                    old_lines.push(line.to_owned());
                } else if line.line_type == Some(parser::LineType::Inserts) && old_lines.len() > 0 {
                    new_lines.push(line.to_owned());
                } else {
                    eprintln!("unknown state in html side-by-side generator");
                    self.process_change_block(file, &mut file_html, &mut old_lines, &mut new_lines);
                }

            }

            self.process_change_block(file, &mut file_html, &mut old_lines, &mut new_lines);
        });

        file_html

    }

    fn process_change_block(
        &self,
        file: &parser::File,
        file_html: &mut SideBySideFile,
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

            let common = min(old_lines.len(), new_lines.len());
            let max = max(old_lines.len(), new_lines.len());

            let mut j = 0;
            let mut old_line;
            let mut new_line;
            while j < common {
                old_line = Some(&old_lines[j]);
                new_line = Some(&new_lines[j]);

                let diff = utils::diff_highlight(
                    &self.config,
                    Some(&self.diff_matcher),
                    &old_line.as_ref().unwrap().content,
                    &new_line.as_ref().unwrap().content,
                );

                file_html.left +=
                    &self.generate_line_html(
                        file.is_combined, &delete_type,
                        old_line.as_ref().and_then(|v| v.old_number),
                        diff.first.line, Some(diff.first.prefix),
                    );
                file_html.right +=
                    &self.generate_line_html(
                        file.is_combined, &insert_type,
                        new_line.as_ref().and_then(|v| v.new_number),
                        diff.second.line, Some(diff.second.prefix),
                    );

                j += 1;
            }

            if max > common {
                let old_slice = &old_lines[common..];
                let new_slice = &new_lines[common..];

                let html = self.process_lines(file.is_combined, &old_slice, &new_slice);
                file_html.left += &html.left;
                file_html.right += &html.right;
            }

        });

        *old_lines = Vec::new();
        *new_lines = Vec::new();

    }

    fn process_lines(
        &self, is_combined: bool,
        old_lines: &[parser::Line], new_lines: &[parser::Line]
    ) -> SideBySideFile {
        let mut file_html = SideBySideFile::new();

        let max_lines_number = max(old_lines.len(), new_lines.len());

        for i in 0..max_lines_number {
            let old_line = old_lines.get(i);
            let new_line = new_lines.get(i);
            let mut old_content = None;
            let mut new_content = None;
            let mut old_prefix = None;
            let mut new_prefix = None;

            if let Some(old_line) = old_line {
                old_content = Some(escape(&old_line.content[1..]).to_string());
                old_prefix = Some(&old_line.content[0..1])
            }

            if let Some(new_line) = new_line {
                new_content = Some(escape(&new_line.content[1..]).to_string());
                new_prefix = Some(&new_line.content[0..1])
            }

            if old_line.is_some() && new_line.is_some() {
                file_html.left += &self.generate_line_html(
                    is_combined,
                    old_line.as_ref().unwrap().line_type.as_ref().unwrap(),
                    old_line.as_ref().unwrap().old_number,
                    old_content.unwrap(), old_prefix,
                );
                file_html.right += &self.generate_line_html(
                    is_combined,
                    new_line.as_ref().unwrap().line_type.as_ref().unwrap(),
                    new_line.as_ref().unwrap().old_number,
                    new_content.unwrap(), new_prefix,
                );
            } else if old_line.is_some() {
                file_html.left += &self.generate_line_html(
                    is_combined,
                    old_line.as_ref().unwrap().line_type.as_ref().unwrap(),
                    old_line.as_ref().unwrap().old_number,
                    old_content.unwrap(), old_prefix,
                );
                file_html.right += &self.generate_line_html(
                    is_combined,
                    &parser::LineType::Context,
                    None, "".to_owned(), None,
                );
            } else if new_line.is_some() {
                file_html.left += &self.generate_line_html(
                    is_combined,
                    &parser::LineType::Context,
                    None, "".to_owned(), None,
                );
                file_html.right += &self.generate_line_html(
                    is_combined,
                    new_line.as_ref().unwrap().line_type.as_ref().unwrap(),
                    new_line.as_ref().unwrap().new_number,
                    new_content.unwrap(), new_prefix,
                );
            } else {
                eprintln!("Unknown path.");
            }

        }

        file_html
    }

    fn generate_line_html(
        &self,
        is_combined: bool,
        line_type: &parser::LineType,
        number: Option<usize>,
        content: String,
        possible_prefix: Option<&str>,
    ) -> String {
        let mut line_class = "d2h-code-side-linenumber".to_owned();
        let mut content_class = "d2h-code-side-line".to_owned();
        let mut line_type = utils::get_line_type_class(line_type).to_owned();

        if number.is_none() && content == "" {
            line_class += " d2h-code-side-emptyplaceholder";
            content_class += " d2h-code-side-emptyplaceholder";
            line_type += " d2h-emptyplaceholder";
        }

        let ( prefix, line_without_prefix, ) = match possible_prefix {
            Some(prefix) => ( prefix, content, ),
            _ => {
                let line_with_prefix = utils::separate_prefix(is_combined, &content);
                ( line_with_prefix.prefix, line_with_prefix.line.to_owned(), )
            }
        };

        self.handlebars.render("generic-line", &json!({
            "type": line_type,
            "lineClass": line_class,
            "contentClass": content_class,
            "prefix": prefix.to_owned(),
            "content": line_without_prefix,
            "lineNumber": number,
        })).unwrap()
    }

    fn generate_empty_diff(&self) -> SideBySideFile {
        let mut file_html = SideBySideFile::new();
        file_html.left += &utils::generate_empty_diff(&self.handlebars, "d2h-code-line");
        file_html
    }

}

#[derive(Debug, Serialize)]
pub struct SideBySideFile {
    pub left: String,
    pub right: String,
}

impl SideBySideFile {
    fn new() -> SideBySideFile {
        SideBySideFile {
            left: String::new(),
            right: String::new(),
        }
    }
}