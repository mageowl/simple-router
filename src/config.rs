use serde::Deserialize;
use xml::ParserConfig;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub out: OutConfig,
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub xml: XmlConfig,
    #[serde(default)]
    pub js: JsConfig,
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

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct JsConfig {
    pub update_anchors: bool,
    pub not_found: String,
}

impl JsConfig {
    pub fn get_code(&self) -> String {
        format!(
            r#"const config = {{
    updateAnchors: {update_anchors},
    notFound: "{not_found}",
}};

"#,
            update_anchors = self.update_anchors,
            not_found = self
                .not_found
                .strip_suffix(".html")
                .unwrap_or(&self.not_found)
        )
    }
}

impl Default for JsConfig {
    fn default() -> Self {
        Self {
            update_anchors: true,
            not_found: String::from("404.html"),
        }
    }
}
