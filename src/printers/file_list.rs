use handlebars::Handlebars;

use crate::parse;

use super::utils;

static FILE_SUMMARY_WRAPPER: &'static str = include_str!("../templates/file-summary-wrapper.hbs");
static FILE_SUMMARY_LINE: &'static str = include_str!("../templates/file-summary-line.hbs");

pub struct FileListPrinter {
    handlebars: Handlebars,
}

impl FileListPrinter {
    pub fn new() -> FileListPrinter {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("wrapper", FILE_SUMMARY_WRAPPER)
            .unwrap();
        handlebars
            .register_template_string("line", FILE_SUMMARY_LINE)
            .unwrap();
        FileListPrinter {
            handlebars: handlebars,
        }
    }

    pub fn render(&mut self, files: &Vec<parse::File>) -> String {
        let file_list = files
            .iter()
            .map(|file| {
                self.handlebars.render(
                    "line",
                    &json!({
                        "fileHtmlId": utils::get_html_id(file),
                        "fileName": utils::get_diff_name(file),
                        "deletedLines": format!("-{}", file.deleted_lines),
                        "addedLines": format!("+{}", file.added_lines),
                        "fileIcon": utils::get_file_type_icon(file).to_owned(),
                    }),
                )
            })
            .map(|v| v.unwrap())
            .collect::<Vec<String>>()
            .join("\n");

        self.handlebars
            .render(
                "wrapper",
                &json!({
                    "filesNumber": files.len(),
                    "files": file_list,
                }),
            )
            .unwrap()
    }
}
