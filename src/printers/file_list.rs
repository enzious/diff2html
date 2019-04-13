use handlebars::Handlebars;

use crate::parser;

use super::utils;

static FILE_SUMMARY_WRAPPER: &'static str = include_str!("../templates/file-summary-wrapper.hbs");
static FILE_SUMMARY_LINE: &'static str = include_str!("../templates/file-summary-line.hbs");

pub fn generate_file_list_summary(files: &Vec<parser::File>) -> String {

    let mut reg = Handlebars::new();
    reg.register_template_string("wrapper", FILE_SUMMARY_WRAPPER).unwrap();
    reg.register_template_string("line", FILE_SUMMARY_LINE).unwrap();

    let file_list = files.iter().map(|file| {
        reg.render("line", &json!({
            "fileHtmlId": utils::get_html_id(file),
            "fileName": utils::get_diff_name(file),
            "deletedLines": format!("-{}", file.deleted_lines),
            "addedLines": format!("+{}", file.added_lines),
            "fileIcon": utils::get_file_type_icon(file).to_owned(),
        }))
    }).map(|v| v.unwrap()).collect::<Vec<String>>().join("\n");

    reg.render("wrapper", &json!({
        "filesNumber": files.len(),
        "files": file_list,
    })).unwrap()

}