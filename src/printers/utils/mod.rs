use std::fmt;
use std::hash::{ Hash, Hasher, };

use handlebars::Handlebars;
use seahash;
use v_htmlescape::escape;

use crate::config::Diff2HtmlConfig;
use crate::parser;

pub mod rematch;

static SEPARATOR: &str = "/";

static ICON_FILE_ADDED: &'static str = include_str!("../../templates/icon-file-added.hbs");
static ICON_FILE_CHANGED: &'static str = include_str!("../../templates/icon-file-changed.hbs");
static ICON_FILE_DELETED: &'static str = include_str!("../../templates/icon-file-deleted.hbs");
static ICON_FILE_RENAMED: &'static str = include_str!("../../templates/icon-file-renamed.hbs");
static TAG_FILE_ADDED: &'static str = include_str!("../../templates/tag-file-added.hbs");
static TAG_FILE_CHANGED: &'static str = include_str!("../../templates/tag-file-changed.hbs");
static TAG_FILE_DELETED: &'static str = include_str!("../../templates/tag-file-deleted.hbs");
static TAG_FILE_RENAMED: &'static str = include_str!("../../templates/tag-file-renamed.hbs");

pub fn get_html_id(file: &parser::File) -> String {
    let diff_name = get_diff_name(file);
    format!("d2h-{}", seahash::hash(diff_name.as_bytes()).to_string())
}

pub fn get_diff_name(file: &parser::File) -> String {

    let old_filename = unify_path(&file.old_name);
    let new_filename = unify_path(&file.new_name);

    if 
        old_filename != new_filename
        && old_filename.as_ref().map(|name| {
            !is_dev_null_name(name)
        }) == Some(true)
        && new_filename.as_ref().map(|name| {
            !is_dev_null_name(name)
        }) == Some(true)
    {

        let mut prefix_paths = Vec::new();
        let mut suffix_paths = Vec::new();

        let old_filename_parts = old_filename.as_ref().unwrap()
            .split(SEPARATOR)
            .collect::<Vec<&str>>();
        let new_filename_parts = new_filename.as_ref().unwrap()
            .split(SEPARATOR)
            .collect::<Vec<&str>>();

        let old_filename_parts_size = old_filename_parts.len();
        let new_filename_parts_size = new_filename_parts.len();

        let mut i = 0;
        let mut j = old_filename_parts_size - 1;
        let mut k = new_filename_parts_size - 1;

        while i < j && i < k {
            if old_filename_parts[i] == new_filename_parts[i] {
                prefix_paths.push(new_filename_parts[i]);
                i += 1;
            } else {
                break;
            }
        }

        while j > i && k > i {
            if old_filename_parts[j] == new_filename_parts[k] {
                suffix_paths.insert(0, new_filename_parts[k]);
                j -= 1;
                k -= 1;
            } else {
                break;
            }
        }

        let final_prefix = prefix_paths.join(SEPARATOR);
        let final_suffix = suffix_paths.join(SEPARATOR);

        let old_remaining_path = old_filename_parts[i..j + 1].join(SEPARATOR);
        let new_remaining_path = new_filename_parts[i..k + 1].join(SEPARATOR);

        if final_prefix.len() != 0 && final_suffix.len() != 0 {
            return final_prefix + SEPARATOR + "{" + &old_remaining_path + " → "
                + &new_remaining_path + "}" + SEPARATOR + &final_suffix;
        } else if final_prefix.len() != 0 {
            return final_prefix + SEPARATOR + "{" + &old_remaining_path + " → "
                + &new_remaining_path + "}";
        } else if final_suffix.len() != 0 {
            return "{".to_owned() + &old_remaining_path + " → " + &new_remaining_path + "}"
                + SEPARATOR + &final_suffix;
        }

        return old_filename.unwrap() + " → " + &new_filename.unwrap();

    } else if 
        new_filename.as_ref().map(|name| {
            !is_dev_null_name(name)
        }) == Some(true)
    {
        return new_filename.unwrap();
    } else if old_filename.is_some() {
        return old_filename.unwrap();
    }

    "uknown/file/path".to_owned()
}

fn unify_path(path: &Option<String>) -> Option<String> {
    if let Some(path) = path {
        return Some(path.replace("\\", "/"));
    }
    return path.to_owned();
}

fn is_dev_null_name(name: &str) -> bool {
    name.contains("dev/null")
}

#[derive(PartialEq)]
pub struct Difference(difference::Difference);

impl Clone for Difference {
    fn clone(&self) -> Self {
        Difference(match &self.0 {
            difference::Difference::Same(content) => {
                difference::Difference::Same(content.to_owned())
            }
            difference::Difference::Add(content) => {
                difference::Difference::Add(content.to_owned())
            }
            difference::Difference::Rem(content) => {
                difference::Difference::Rem(content.to_owned())
            }
        })
    }
}

impl fmt::Debug for Difference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let content = match &self.0 {
            difference::Difference::Same(ref content)
            | difference::Difference::Add(ref content)
            | difference::Difference::Rem(ref content) => {
                content
            }
        };
        write!(f, "Difference {{ {} }}", content)
    }
}

impl Hash for Difference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            difference::Difference::Same(content) => format!("same:{}", &content).hash(state),
            difference::Difference::Add(content) => format!("add:{}", &content).hash(state),
            difference::Difference::Rem(content) => format!("rem:{}", &content).hash(state),
        }
    }
}

pub fn get_line_matcher() -> rematch::Rematcher<parser::Line> {
    rematch::Rematcher::new(|a: &parser::Line, b: &parser::Line| {
        let amod = &a.content[1..];
        let bmod = &b.content[1..];
        rematch::distance(amod, bmod)
    })
}

pub fn get_difference_matcher() -> rematch::Rematcher<Difference> {
    rematch::Rematcher::new(|a: &Difference, b: &Difference| {
        let amod = match &a.0 {
            difference::Difference::Same(content)
            | difference::Difference::Add(content)
            | difference::Difference::Rem(content) => {
                content
            }
        };
        let bmod = match &b.0 {
            difference::Difference::Same(content)
            | difference::Difference::Add(content)
            | difference::Difference::Rem(content) => {
                content
            }
        };
        rematch::distance(&amod, &bmod)
    })
}

pub fn diff_highlight<'a>(
    config: &Diff2HtmlConfig,
    matcher: Option<&rematch::Rematcher<Difference>>,
    diff_line1: &'a str,
    diff_line2: &'a str,
) -> Highlighted<'a> {

    // TODO: idk
    let mut matcher = matcher;
    let matcher_alt = if matcher.is_none() {
        Some(get_difference_matcher())
    } else {
        None
    };
    if matcher.is_none() {
        matcher = matcher_alt.as_ref();
    };

    let line_prefix1;
    let line_prefix2;
    let unprefixed_line1;
    let unprefixed_line2;

    let prefix_size = if config.is_combined {
        2
    } else {
        1
    };

    line_prefix1 = &diff_line1[0..prefix_size];
    line_prefix2 = &diff_line2[0..prefix_size];
    unprefixed_line1 = &diff_line1[prefix_size..];
    unprefixed_line2 = &diff_line2[prefix_size..];
    
    if
        unprefixed_line1.len() > config.max_line_length_highlight
        || unprefixed_line2.len() > config.max_line_length_highlight
    {
        return Highlighted {
            first: HighlightedLine {
                prefix: line_prefix1,
                // TODO: Escape.
                line: unprefixed_line1.to_owned(),
            },
            second: HighlightedLine {
                prefix: line_prefix2,
                // TODO: Escape.
                line: unprefixed_line2.to_owned(),
            }
        };
    }

    let diffs: Vec<Difference> = if config.matching != "chars" {
        difference::Changeset::new(unprefixed_line1, unprefixed_line2, " ")
    } else {
        difference::Changeset::new(unprefixed_line1, unprefixed_line2, "")
    }.diffs.drain(..).map(|v| Difference(v)).collect();

    let mut changed_words = Vec::new();
    if config.matching == "words" {
        let threshold = config.match_words_threshold;

        let removed = diffs.iter().filter(|diff| {
            match &diff.0 {
                difference::Difference::Rem(_) => true,
                _ => false,
            }
        }).collect();

        let added = diffs.iter().filter(|diff| {
            match &diff.0 {
                difference::Difference::Add(_) => true,
                _ => false,
            }
        }).collect();

        let chunks = matcher.unwrap().matches_ref(&added, &removed);
        chunks.iter().for_each(|chunk| {
            if chunk[0].len() == 1 && chunk[1].len() == 1 {
                let dist = rematch::distance(
                    match &chunk[0][0].0 {
                        difference::Difference::Same(ref s)
                        | difference::Difference::Add(ref s)
                        | difference::Difference::Rem(ref s) => s,
                    },
                    match &chunk[1][0].0 {
                        difference::Difference::Same(ref s)
                        | difference::Difference::Add(ref s)
                        | difference::Difference::Rem(ref s) => s,
                    },
                );
                if dist < threshold {
                    changed_words.push(chunk[0][0].to_owned());
                    changed_words.push(chunk[1][0].to_owned());
                }
            }
        });

    }

    let mut delete_line = Vec::new();
    let mut insert_line = Vec::new();
    diffs.iter().for_each(|part| {

        let add_class = if changed_words.contains(&part) {
            r#" class="d2h-change""#
        } else {
            ""
        };

        match &part.0 {
            difference::Difference::Add(ref s) => {
                insert_line.push(format!("<{}{}>{}</{}>",
                        "ins", add_class, escape(s).to_string(), "ins"));
            },
            difference::Difference::Rem(ref s) => {
                delete_line.push(format!("<{}{}>{}</{}>",
                        "del", add_class, escape(s).to_string(), "del"));
            },
            difference::Difference::Same(ref s) => {
                let escaped = escape(s).to_string();
                insert_line.push(escaped.to_owned());
                delete_line.push(escaped);
            },
        };

    });

    let join = if config.matching == "words" {
        " "
    } else {
        ""
    };
    let delete_line = delete_line.join(join);
    let insert_line = insert_line.join(join);

    return Highlighted {
        first: HighlightedLine {
            prefix: line_prefix1,
            // TODO: Escape.
            line: delete_line,
        },
        second: HighlightedLine {
            prefix: line_prefix2,
            // TODO: Escape.
            line: insert_line,
        }
    };

}

pub struct Highlighted<'a> {
    pub first: HighlightedLine<'a>,
    pub second: HighlightedLine<'a>,
}

pub struct HighlightedLine<'a> {
    pub prefix: &'a str,
    pub line: String,
}

pub struct SeparatedLine<'a> {
    pub prefix: &'a str,
    pub line: &'a str,
}

pub fn separate_prefix<'a>(is_combined: bool, line: &'a str) -> SeparatedLine<'a> {
    if line == "" {
        SeparatedLine {
            prefix: "",
            line: "",
        }
    } else if is_combined {
        SeparatedLine {
            prefix: &line[0..2],
            line: &line[2..],
        }
    } else {
        SeparatedLine {
            prefix: &line[0..1],
            line: &line[1..],
        }
    }
}

pub fn get_file_type_icon(file: &parser::File) -> &str {
    let mut partial = ICON_FILE_CHANGED;

    if file.is_rename {
        partial = ICON_FILE_RENAMED;
    } else if file.is_copy {
        partial = ICON_FILE_RENAMED;
    } else if file.is_new {
        partial = ICON_FILE_ADDED;
    } else if file.is_deleted {
        partial = ICON_FILE_DELETED;
    } else if file.new_name != file.old_name {
        // If file is not Added, not Deleted and the names changed it must be a rename :)
        partial = ICON_FILE_RENAMED;
    }

    partial
}

pub fn get_line_type_class(line_type: &parser::LineType) -> &str {
    match line_type {
        parser::LineType::Inserts => "d2h-ins",
        parser::LineType::Deletes => "d2h-del",
        parser::LineType::InsertChanges => "d2h-ins d2h-change",
        parser::LineType::DeleteChanges => "d2h-del d2h-change",
        parser::LineType::Context => "d2h-cntx",
        // parser::LineType::Info => "d2h-info",
    }
}

pub fn get_line_type_tag(file: &parser::File) -> &str {
    let mut partial = TAG_FILE_CHANGED;

    if file.is_rename {
        partial = TAG_FILE_RENAMED;
    } else if file.is_copy {
        partial = TAG_FILE_RENAMED;
    } else if file.is_new {
        partial = TAG_FILE_ADDED;
    } else if file.is_deleted {
        partial = TAG_FILE_DELETED;
    } else if file.new_name != file.old_name {
        // If file is not Added, not Deleted and the names changed it must be a rename :)
        partial = TAG_FILE_RENAMED;
    }

    partial
}

pub fn make_column_line_number_html(
    handlebars: &Handlebars, header: &str,
    line_class: &str, content_class: &str,
) -> String {
    handlebars.render("generic-column-line-number", &json!({
        "blockHeader": header,
        "lineClass": line_class,
        "contentClass": content_class,
    })).unwrap()
}

pub fn generate_empty_diff(handlebars: &Handlebars, content_class: &str) -> String {
    handlebars.render("generic-empty-diff", &json!({
        "contentClass": content_class,
    })).unwrap()
}