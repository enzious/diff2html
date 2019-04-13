use core::str::Split;
use std::borrow::Cow;

use regex::Regex;

static OLD_FILE_NAME_HEADER: &str = "--- ";
static NEW_FILE_NAME_HEADER: &str = "+++ ";
static HUNK_HEADER_PREFIX: &str = "@@";

#[derive(Debug)]
pub struct DiffParser {

}

impl DiffParser {

    pub fn new() -> DiffParser {
        DiffParser {}
    }

    pub fn parse_diff(&mut self, diff: &str) -> Vec<File> {

        let mut state = ParseState::new();

        let diff: Cow<'_, str> = Cow::Borrowed(diff);

        lazy_static! {

            static ref NO_NEWLINE: Regex = Regex::new(r#"\\ No newline at end of file"#).unwrap();
            static ref REPLACE_LINE: Regex = Regex::new(r#"\r\n?"#).unwrap();

            // Diff
            static ref OLD_MODE: Regex = Regex::new(r#"old mode (\d{6})"#).unwrap();
            static ref NEW_MODE: Regex = Regex::new(r#"new mode (\d{6})"#).unwrap();
            static ref DELETED_FILE_MODE: Regex = Regex::new(r#"deleted file mode (\d{6})"#).unwrap();
            static ref NEW_FILE_MODE: Regex = Regex::new(r#"new file mode (\d{6})"#).unwrap();

            static ref COPY_FROM: Regex = Regex::new(r#"copy from "?(.+)"?"#).unwrap();
            static ref COPY_TO: Regex = Regex::new(r#"copy to "?(.+)"?"#).unwrap();

            static ref RENAME_FROM: Regex = Regex::new(r#"rename from "?(.+)"?"#).unwrap();
            static ref RENAME_TO: Regex = Regex::new(r#"rename to "?(.+)"?"#).unwrap();

            static ref SIMILARITY_INDEX: Regex = Regex::new(r#"similarity index (\d+)%"#).unwrap();
            static ref DISSIMILARITY_INDEX: Regex = Regex::new(r#"dissimilarity index (\d+)%"#).unwrap();
            static ref INDEX: Regex = Regex::new(r#"index ([0-9a-z]+)\.\.([0-9a-z]+)\s*(\d{6})?"#).unwrap();

            static ref BINARY_FILES: Regex = Regex::new(r#"Binary files (.*) and (.*) differ"#).unwrap();
            static ref BINARY_DIFF: Regex = Regex::new(r#"GIT binary patch"#).unwrap();

            // Combined Diff
            static ref COMBINED_INDEX: Regex = Regex::new(r#"index ([0-9a-z]+),([0-9a-z]+)\.\.([0-9a-z]+)"#).unwrap();
            static ref COMBINED_MODE: Regex = Regex::new(r#"mode (\d{6}),(\d{6})\.\.(\d{6})"#).unwrap();
            static ref COMBINED_NEW_FILE: Regex = Regex::new(r#"new file mode (\d{6})"#).unwrap();
            static ref COMBINED_DELETED_FILE: Regex = Regex::new(r#"deleted file mode (\d{6}),(\d{6})"#).unwrap();

            static ref GIT_DIFF_START: Regex = Regex::new(r#"^diff --git "?(.+)"? "?(.+)"?"#).unwrap();

        }

        let diff = NO_NEWLINE.replace_all(&diff, "");
        let diff = REPLACE_LINE.replace_all(&diff, "\n");

        let diff_lines: Split<&str> = diff.split("\n");
        let diff_lines: Vec<&str> = diff_lines.collect();

        for (i, line) in diff_lines.iter().enumerate() {

            if *line == "" || line.starts_with("*") {
                continue;
            }

            // Collect some lines.
            let prev_line = if i == 0 {
                None
            } else {
                diff_lines.get(i - 1)
            };
            let next_line = diff_lines.get(i + 1);
            let after_next_line = diff_lines.get(i + 2);

            if line.starts_with("diff") {
                //println!("got new file");
                state.start_file();

                let captures = GIT_DIFF_START.captures(line);
                if let Some(captures) = captures {
                    state.possible_old_name = get_filename(None, captures.get(1).unwrap().as_str(), None);
                    state.possible_new_name = get_filename(None, captures.get(2).unwrap().as_str(), None);
                }

                state.current_file.as_mut().unwrap().is_git_diff = true;
                continue;
            }

            if
                // If we do not have a file yet, create one.
                state.current_file.is_none()
                || (
                    // We already have some file in progress.
                    !state.current_file.as_ref().unwrap().is_git_diff
                    && (
                        // If we get to an old file path header line
                        line.starts_with(OLD_FILE_NAME_HEADER)
                        // And it's followed by the new file path header...
                        && next_line.unwrap().starts_with(NEW_FILE_NAME_HEADER)
                        // ...and the hunk header line.
                        && after_next_line.unwrap().starts_with(HUNK_HEADER_PREFIX)
                    )
                )
            {
                state.start_file();
            }

            if
                (
                    next_line.is_some()
                    && line.starts_with(OLD_FILE_NAME_HEADER)
                    && next_line.unwrap().starts_with(NEW_FILE_NAME_HEADER)
                )
                ||
                (
                    prev_line.is_some()
                    && line.starts_with(NEW_FILE_NAME_HEADER)
                    && prev_line.unwrap().starts_with(OLD_FILE_NAME_HEADER)
                )
            {

                if
                    state.current_file.is_some()
                    && state.current_file.as_ref().unwrap().old_name.is_none()
                    && line.starts_with("--- ")
                {
                    state.current_file.as_mut().map(|file| {
                        file.old_name = get_src_filename(line, None);
                        file.language = get_extension(file.old_name.as_ref().unwrap(), file.language.as_ref().map(|v| v.as_str()));
                        file
                    });
                    continue;
                }

                if
                    state.current_file.is_some()
                    && state.current_file.as_ref().unwrap().new_name.is_none()
                    && line.starts_with("+++ ")
                {
                    state.current_file.as_mut().map(|file| {
                        file.new_name = get_dst_filename(line, None);
                        file.language = get_extension(file.new_name.as_ref().unwrap(), file.language.as_ref().map(|v| v.as_str()));
                        file
                    });
                    continue;
                }

            }

            if
                (state.current_file.is_some() && line.starts_with(HUNK_HEADER_PREFIX))
                || (
                    state.current_file.as_ref().map(|file| {
                        file.is_git_diff && file.old_name.is_some() && file.new_name.is_some()
                    }) == Some(true)
                    && state.current_block.is_none()
                )
            {
                state.start_block(line);
                continue;
            }

            if
                state.current_block.is_some()
                && (
                    line.starts_with("+") || line.starts_with("-") || line.starts_with(" ")
                )
            {
                state.create_line(line);
                continue;
            }

            let does_not_exist_hunk_header = exist_hunk_header(line, &diff_lines, i);

            /*
             * Git diffs provide more information regarding files modes, renames, copies,
             * commits between changes and similarity indexes
             */
            if let Some(captures) = OLD_MODE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.old_mode = Some(vec![captures.get(1).unwrap().as_str().to_owned()]);
                });
            } else if let Some(captures) = NEW_MODE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.new_mode = captures.get(1).map(|v| v.as_str().to_owned());
                });
            } else if let Some(captures) = DELETED_FILE_MODE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.deleted_file_mode = captures.get(1).map(|v| v.as_str().to_owned());
                    file.is_deleted = true;
                });
            } else if let Some(captures) = NEW_FILE_MODE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.new_file_mode = captures.get(1).map(|v| v.as_str().to_owned());
                    file.is_new = true;
                });
            } else if let Some(captures) = COPY_FROM.captures(line) {
                state.current_file.as_mut().map(|file| {
                    if does_not_exist_hunk_header {
                        file.old_name = captures.get(1).map(|v| v.as_str().to_owned());
                    }
                    file.is_copy = true;
                });
            } else if let Some(captures) = COPY_TO.captures(line) {
                state.current_file.as_mut().map(|file| {
                    if does_not_exist_hunk_header {
                        file.new_name = captures.get(1).map(|v| v.as_str().to_owned());
                    }
                    file.is_copy = true;
                });
            } else if let Some(captures) = RENAME_FROM.captures(line) {
                state.current_file.as_mut().map(|file| {
                    if does_not_exist_hunk_header {
                        file.old_name = captures.get(1).map(|v| v.as_str().to_owned());
                    }
                    file.is_rename = true;
                });
            } else if let Some(captures) = RENAME_TO.captures(line) {
                state.current_file.as_mut().map(|file| {
                    if does_not_exist_hunk_header {
                        file.new_name = captures.get(1).map(|v| v.as_str().to_owned());
                    }
                    file.is_rename = true;
                });
            } else if let Some(captures) = BINARY_FILES.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.is_binary = true;
                    file.old_name = get_filename(None, captures.get(1).map(|v| v.as_str()).unwrap(), None);
                    file.new_name = get_filename(None, captures.get(2).map(|v| v.as_str()).unwrap(), None);
                });
                state.start_block("Binary file");
            } else if let Some(_captures) = BINARY_DIFF.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.is_binary = true;
                });
                state.start_block(line);
            } else if let Some(captures) = SIMILARITY_INDEX.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.unchanged_percentage = captures.get(1).map(|v| v.as_str().parse().unwrap());
                });
            } else if let Some(captures) = DISSIMILARITY_INDEX.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.changed_percentage = captures.get(1).map(|v| v.as_str().parse().unwrap());
                });
            } else if let Some(captures) = INDEX.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.checksum_before = Some(vec![captures.get(1).unwrap().as_str().to_owned()]);
                    file.checksum_after = captures.get(2).map(|v| v.as_str().to_owned());
                    if let Some(mode) = captures.get(3) {
                        file.mode = Some(mode.as_str().to_owned());
                    }
                });
            } else if let Some(captures) = COMBINED_INDEX.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.checksum_before = Some(vec![
                        captures.get(1).unwrap().as_str().to_owned(),
                        captures.get(2).unwrap().as_str().to_owned(),
                    ]);
                    file.checksum_after = captures.get(3).map(|v| v.as_str().to_owned());
                });
            } else if let Some(captures) = COMBINED_MODE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.old_mode = Some(vec![
                        captures.get(1).unwrap().as_str().to_owned(),
                        captures.get(2).unwrap().as_str().to_owned(),
                    ]);
                    file.new_mode = captures.get(3).map(|v| v.as_str().to_owned());
                });
            } else if let Some(captures) = COMBINED_NEW_FILE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.new_file_mode = captures.get(1).map(|v| v.as_str().to_owned());
                    file.is_new = true;
                });
            } else if let Some(captures) = COMBINED_DELETED_FILE.captures(line) {
                state.current_file.as_mut().map(|file| {
                    file.deleted_file_mode = captures.get(1).map(|v| v.as_str().to_owned());
                    file.is_deleted = true;
                });
            }

            //println!("{}, {}", i, line);
        }

        state.save_block();
        state.save_file();

        //println!("{:#?}", &state.files);

        state.files
    }

}

struct ParseState {
    files: Vec<File>,
    current_file: Option<File>,
    current_block: Option<Block>,
    // TODO: Why is this not used.
    //current_line: Option<Line>,
    old_line: Option<usize>,
    old_line_2: Option<usize>,
    new_line: Option<usize>,
    possible_old_name: Option<String>,
    possible_new_name: Option<String>,
}

impl ParseState {

    fn new() -> ParseState {
        ParseState {
            files: Vec::new(),
            current_file: None,
            current_block: None,
            old_line: None,
            old_line_2: None,
            new_line: None,
            possible_old_name: None,
            possible_new_name: None,
        }
    }

    fn start_file(&mut self) {
        self.save_block();
        self.save_file();

        let file = File::new();
        self.current_file = Some(file);
    }

    fn save_file(&mut self) {

        if
            self.current_file.is_some()
            && {
                let file = self.current_file.as_mut().unwrap();
                if file.old_name.is_none() {
                    file.old_name = self.possible_old_name.take();
                }
                if file.new_name.is_none() {
                    file.new_name = self.possible_new_name.take();
                }
                file.new_name.is_some()
            }
        {
            self.files.push(self.current_file.take().unwrap());
        }

        self.possible_old_name = None;
        self.possible_new_name = None;

    }

    fn start_block(&mut self, line: &str) {
        self.save_block();

        lazy_static! {
            static ref RANGE1: Regex = Regex::new(r#"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@.*"#).unwrap();
            static ref RANGE2: Regex = Regex::new(r#"@@@ -(\d+)(?:,\d+)? -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@@.*"#).unwrap();
        }

        let captures = RANGE1.captures(line);
        match captures {
            Some(captures) => {
                self.current_file.as_mut().map(|file| file.is_combined = false);
                self.old_line = captures.get(1).map(|v| v.as_str().parse().unwrap());
                self.new_line = captures.get(2).map(|v| v.as_str().parse().unwrap());
            },
            _ => {
                let captures = RANGE2.captures(line);
                match captures {
                    Some(captures) => {
                        self.current_file.as_mut().map(|file| file.is_combined = true);
                        self.old_line = captures.get(1).map(|v| v.as_str().parse().unwrap());
                        self.old_line_2 = captures.get(2).map(|v| v.as_str().parse().unwrap());
                        self.new_line = captures.get(3).map(|v| v.as_str().parse().unwrap());
                    },
                    _ => {
                        if line.starts_with(HUNK_HEADER_PREFIX) {
                            eprintln!("Failed to parse lines, starting in 0!");
                        }
                        self.old_line = Some(0);
                        self.new_line = Some(0);
                        self.current_file.as_mut().map(|file| file.is_combined = false);
                    }
                }
            }
        }

        let mut block = Block::new();
        block.header = Some(line.to_owned());
        self.current_block = Some(block);
    }

    fn save_block(&mut self) {
        if let Some(block) = self.current_block.take() {
            if let Some(ref mut file) = self.current_file {
                file.blocks.push(block);
            }
        }
    }

    fn create_line(&mut self, line: &str) {
        let mut line = Line::new(line.to_owned());

        let add_line_prefixes = self.current_file.as_ref().map(|v| {
            if !v.is_combined {
                "+"
            } else {
                "++"
            }
        }).unwrap();

        let delete_line_prefixes = self.current_file.as_ref().map(|v| {
            if !v.is_combined {
                "-"
            } else {
                "--"
            }
        }).unwrap();

        if line.content.starts_with(add_line_prefixes) {
            self.current_file.as_mut().unwrap().added_lines += 1;
            line.line_type = Some(LineType::Inserts);
            line.old_number = None;
            line.new_number = self.new_line.to_owned();
            self.new_line.as_mut().map(|v| *v += 1);
        } else if line.content.starts_with(delete_line_prefixes) {
            self.current_file.as_mut().unwrap().deleted_lines += 1;
            line.line_type = Some(LineType::Deletes);
            line.old_number = self.old_line.to_owned();
            self.old_line.as_mut().map(|v| *v += 1);
            line.new_number = None;
        } else {
            line.line_type = Some(LineType::Context);
            line.old_number = self.old_line.to_owned();
            self.old_line.as_mut().map(|v| *v += 1);
            line.new_number = self.new_line.to_owned();
            self.new_line.as_mut().map(|v| *v += 1);
        }

        self.current_block.as_mut().unwrap().lines.push(line);
    }

}

#[derive(Debug, Serialize)]
pub struct File {
    pub old_name: Option<String>,
    pub new_name: Option<String>,
    pub is_combined: bool,
    pub is_git_diff: bool,
    pub language: Option<String>,
    pub blocks: Vec<Block>,
    pub added_lines: usize,
    pub deleted_lines: usize,
    pub mode: Option<String>,
    pub old_mode: Option<Vec<String>>,
    pub new_mode: Option<String>,
    pub new_file_mode: Option<String>,
    pub deleted_file_mode: Option<String>,
    pub is_deleted: bool,
    pub is_new: bool,
    pub is_copy: bool,
    pub is_rename: bool,
    pub is_binary: bool,
    pub unchanged_percentage: Option<usize>,
    pub changed_percentage: Option<usize>,
    pub checksum_before: Option<Vec<String>>,
    pub checksum_after: Option<String>,
}

impl File {
    fn new() -> File {
        File {
            old_name: None,
            new_name: None,
            is_combined: false,
            is_git_diff: false,
            language: None,
            blocks: Vec::new(),
            added_lines: 0,
            deleted_lines: 0,
            mode: None,
            old_mode: None,
            new_mode: None,
            new_file_mode: None,
            deleted_file_mode: None,
            is_deleted: false,
            is_new: false,
            is_copy: false,
            is_rename: false,
            is_binary: false,
            unchanged_percentage: None,
            changed_percentage: None,
            checksum_before: None,
            checksum_after: None,
        }
    }
}

#[derive(Clone, Debug, Hash, Serialize)]
pub struct Line {
    pub content: String,
    pub line_type: Option<LineType>,
    pub old_number: Option<usize>,
    pub new_number: Option<usize>,
}

impl Line {
    fn new(line: String) -> Line {
        Line {
            content: line,
            line_type: None,
            old_number: None,
            new_number: None,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Serialize)]
pub enum LineType {
    Inserts,
    Deletes,
    Context,
    //Info,
    InsertChanges,
    DeleteChanges,
}

#[derive(Debug, Serialize)]
pub struct Block {
    pub lines: Vec<Line>,
    pub header: Option<String>,
}

impl Block {
    fn new() -> Block {
        Block {
            lines: Vec::new(),
            header: None,
        }
    }
}

fn get_extension(filename: &str, language: Option<&str>) -> Option<String> {
    let name_split: Vec<String> = filename.split(".").map(|v| v.to_string()).collect();
    if name_split.len() > 1 {
        return Some(name_split.get(name_split.len() - 1).unwrap().to_owned());
    }
    language.map(|v| v.to_owned())
}

fn get_src_filename(line: &str, prefix: Option<&str>) -> Option<String> {
    get_filename(Some("---"), line, prefix)
}

fn get_dst_filename(line: &str, prefix: Option<&str>) -> Option<String> {
    get_filename(Some("\\+\\+\\+"), line, prefix)
}

fn get_filename(line_prefix: Option<&str>, line: &str, extra_prefix: Option<&str>) -> Option<String> {

    lazy_static! {
        static ref PREFIXES: Vec<&'static str> = vec!["a/", "b/", "i/", "w/", "c/", "o/"];
        static ref FILENAME_REG: Regex = Regex::new(r#"^"?(.+?)"?$"#).unwrap();
        static ref DATE: Regex = Regex::new(r#"\s+\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)? [-+]\d{4}.*$"#).unwrap();
    }

    //println!("FILENAME: {}", line);

    let mut filename = None;

    let captures = match line_prefix {
        Some(line_prefix) => {
            Regex::new(&format!(r#"^{} "?(.+?)"?$"#, line_prefix)).unwrap().captures(line)
        },
        _ => {
            FILENAME_REG.captures(line)
        },
    };

    if let Some(matches) = captures {

        if matches.len() > 0 {
            filename = Some(matches.get(1).unwrap().as_str().to_owned());
        }

        let mut matching_prefixes = PREFIXES.iter().filter(|p| {
            filename.as_ref().unwrap().contains(*p)
        }).map(|v| v.to_string()).collect::<Vec<String>>();

        if let Some(extra_prefix) = extra_prefix {
            if filename.as_ref().unwrap().contains(extra_prefix) {
                matching_prefixes.push(extra_prefix.to_owned());
            }
        }

        if let Some(prefix) = matching_prefixes.get(0) {
            filename.as_mut().map(|filename| *filename = filename[prefix.len()..].to_string());
        }

        filename.as_mut().map(|filename| *filename = DATE.replace(filename, "").to_string());

    }

    //println!("filename out: {:?}", &filename);

    filename
}

fn exist_hunk_header(line: &str, lines: &Vec<&str>, index: usize) -> bool {
    let mut idx = index;

    while idx < lines.len() - 3 {
        if line.starts_with("diff") {
            return false;
        }

        if
            lines.get(idx).unwrap().starts_with(OLD_FILE_NAME_HEADER)
            && lines.get(idx + 1).unwrap().starts_with(NEW_FILE_NAME_HEADER)
            && lines.get(idx + 2).unwrap().starts_with(HUNK_HEADER_PREFIX)
        {
            return true;
        }

        idx += 1;
    }

    false
}