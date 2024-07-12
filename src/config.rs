use serde::Deserialize;
use xml::ParserConfig;

#[derive(Deserialize)]
pub struct Config {
    pub source: SourceConfig,
    pub out: OutConfig,
    #[serde(default)]
    pub xml: XmlConfig,
}

#[derive(Deserialize)]
pub struct SourceConfig {
    pub path: String,
    #[serde(default = "default_template_path")]
    pub template: String,
}

#[derive(Deserialize)]
pub struct OutConfig {
    pub path: String,
    #[serde(default = "default_js_lib_path")]
    pub lib_file: String,
}

fn default_template_path() -> String {
    String::from("layout.html")
}

fn default_js_lib_path() -> String {
    String::from("simple-router.js")
}

#[derive(Deserialize)]
#[serde(default)]
pub struct XmlConfig {
    pub ignore_comments: bool,
}

impl Default for XmlConfig {
    fn default() -> Self {
        Self {
            ignore_comments: true,
        }
    }
}

impl Into<ParserConfig> for XmlConfig {
    fn into(self) -> ParserConfig {
        ParserConfig {
            ignore_comments: self.ignore_comments,
            ..Default::default()
        }
    }
}
