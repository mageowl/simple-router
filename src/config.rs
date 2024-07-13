use serde::Deserialize;
use xml::ParserConfig;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub out: OutConfig,
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub xml: XmlConfig,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct SourceConfig {
    pub path: String,
    pub template: String,
    pub exclude: Vec<String>,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            path: String::from("."),
            template: String::from("layout.html"),
            exclude: Vec::new(),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct OutConfig {
    pub path: String,
    #[serde(default = "default_js_lib_path")]
    pub lib_file: String,
}

fn default_js_lib_path() -> String {
    String::from("simple-router.js")
}

#[derive(Deserialize, Clone, Copy)]
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
