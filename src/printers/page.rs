use crate::config::Diff2HtmlConfig;
use crate::parse;
use crate::printers::{FileListPrinter, LineByLinePrinter, SideBySidePrinter};

static CSS: &'static str = include_str!("../templates/css.hbs");

pub struct PagePrinter {
    config: Diff2HtmlConfig,
}

impl PagePrinter {
    pub fn new(config: Diff2HtmlConfig) -> PagePrinter {
        PagePrinter { config: config }
    }

    pub fn render(&self, files: &Vec<parse::File>) -> String {
        let summary = if self.config.summary != "hidden" {
            FileListPrinter::new().render(&files)
        } else {
            "".to_owned()
        };

        let content = if self.config.style == "line" {
            LineByLinePrinter::new(self.config.to_owned()).render(&files)
        } else {
            SideBySidePrinter::new(self.config.to_owned()).render(&files)
        };

        format!(
            r#"
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
        "#,
            CSS, &summary, &content
        )
    }
}
