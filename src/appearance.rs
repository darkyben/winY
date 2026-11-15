use gpui::Global;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
use syntect::highlighting::{Color, StyleModifier, Theme, ThemeItem, ThemeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: String,
    pub font: String,
    pub font_size: f32,
    pub tab_size: usize,
    pub highlighting: bool,
    pub word_wrap: bool,
    pub show_tabs: bool,
    pub caret_animation: String,
    pub caret_duration: f32,
    pub opacity: f32,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "winY Black".into(),
            font: "Lilex".into(),
            font_size: 16.,
            tab_size: 4,
            highlighting: true,
            word_wrap: false,
            show_tabs: true,
            caret_animation: "off".into(),
            caret_duration: 120.,
            opacity: 1.,
        }
    }
}
impl Global for Appearance {}
impl Appearance {
    pub fn normalize(&mut self) {
        if !themes().iter().any(|theme| theme.name == self.theme) {
            self.theme = "winY Black".into();
        }
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(4., 128.)
        } else {
            16.
        };
        self.caret_duration = self.caret_duration.clamp(30., 500.);
        self.opacity = self.opacity.clamp(0., 1.);
        self.tab_size = self.tab_size.clamp(1, 8);
    }
    pub fn theme(&self) -> &'static EditorTheme {
        themes()
            .iter()
            .find(|theme| theme.name == self.theme)
            .unwrap_or(&themes()[0])
    }
    pub fn line_height(&self) -> f32 {
        self.font_size * 1.5
    }
    pub fn cell_width(&self) -> f32 {
        self.font_size * 0.6
    }
}
pub struct EditorTheme {
    pub name: String,
    pub background: u32,
    pub foreground: u32,
    pub muted: u32,
    pub surface: u32,
    pub border: u32,
    pub accent: u32,
    pub selection: u32,
    pub syntax: Theme,
    pub swatches: Vec<u32>,
}
fn color(value: Option<&Value>, fallback: u32) -> u32 {
    value
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix('#'))
        .and_then(|s| s.get(..6))
        .and_then(|s| u32::from_str_radix(s, 16).ok())
        .unwrap_or(fallback)
}
fn syntax_color(value: u32) -> Color {
    Color {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
        a: 255,
    }
}
pub fn themes() -> &'static [EditorTheme] {
    static THEMES: OnceLock<Vec<EditorTheme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let mut themes = vec![EditorTheme {
            name: "winY Black".into(),
            background: 0,
            foreground: 0xdddddd,
            muted: 0x777777,
            surface: 0x111111,
            border: 0x333333,
            accent: 0x74ade8,
            selection: 0x253547,
            syntax: ThemeSet::load_defaults().themes["base16-ocean.dark"].clone(),
            swatches: vec![0x74ade8, 0x98c379, 0xc678dd, 0xe5c07b],
        }];
        for source in [
            include_str!("../assets/themes/zed-top20.json"),
            include_str!("../upstream/assets/themes/one/one.json"),
            include_str!("../upstream/assets/themes/ayu/ayu.json"),
            include_str!("../upstream/assets/themes/gruvbox/gruvbox.json"),
        ] {
            let family: Value = serde_json::from_str(source).expect("bundled Zed theme JSON");
            for entry in family["themes"].as_array().expect("Zed theme family") {
                let style = &entry["style"];
                let foreground = color(style.get("editor.foreground"), 0xdddddd);
                let background = color(style.get("editor.background"), 0);
                let accent = color(style.get("text.accent"), foreground);
                let mut syntax = Theme::default();
                syntax.settings.foreground = Some(syntax_color(foreground));
                syntax.settings.background = Some(syntax_color(background));
                let mut swatches = Vec::new();
                for (token, scopes) in [
                    ("comment", "comment"),
                    ("string", "string"),
                    ("number", "constant.numeric"),
                    ("boolean", "constant.language"),
                    ("keyword", "keyword, storage"),
                    ("function", "entity.name.function, support.function"),
                    ("type", "entity.name.type, support.type, support.class"),
                    ("variable", "variable"),
                    ("constant", "constant.other"),
                    ("operator", "keyword.operator"),
                    ("punctuation", "punctuation"),
                    ("tag", "entity.name.tag"),
                    ("attribute", "entity.other.attribute-name"),
                ] {
                    let rgb = color(style["syntax"][token].get("color"), foreground);
                    syntax.scopes.push(ThemeItem {
                        scope: scopes.parse().expect("static syntax scope"),
                        style: StyleModifier {
                            foreground: Some(syntax_color(rgb)),
                            ..Default::default()
                        },
                    });
                    if ["string", "number", "keyword", "function"].contains(&token) {
                        swatches.push(rgb);
                    }
                }
                let selection = color(style["players"][0].get("selection"), accent);
                let blend = |shift: u32| {
                    (((background >> shift) & 255u32) * 3 + ((selection >> shift) & 255u32)) / 4
                };
                themes.push(EditorTheme {
                    name: entry["name"].as_str().expect("theme name").into(),
                    background,
                    foreground,
                    muted: color(style.get("text.muted"), foreground),
                    surface: color(style.get("elevated_surface.background"), background),
                    border: color(style.get("border"), accent),
                    accent,
                    selection: (blend(16) << 16) | (blend(8) << 8) | blend(0),
                    syntax,
                    swatches,
                });
            }
        }
        themes
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_settings_and_old_sessions_round_trip() {
        let old: Appearance = serde_json::from_str(r#"{"font_size":16}"#).unwrap();
        assert_eq!(old.font_size, 16.);
        let settings = Appearance {
            font_size: 16.001,
            word_wrap: true,
            show_tabs: false,
            caret_animation: "neovide".into(),
            opacity: 0.4,
            ..old
        };
        let restored: Appearance =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(settings, restored);
    }
    #[test]
    fn bundled_themes_and_settings_are_valid() {
        assert!(themes().len() >= 100);
        for name in [
            "Catppuccin Mocha",
            "Tokyo Night",
            "Dracula",
            "Nord Dark",
            "Rosé Pine",
            "Zedokai",
            "Fleet Dark",
        ] {
            assert!(themes().iter().any(|t| t.name == name), "missing {name}");
        }
        let names: std::collections::HashSet<_> = themes().iter().map(|t| &t.name).collect();
        assert_eq!(names.len(), themes().len());
        assert!(themes().iter().any(|theme| theme.name == "One Light"));
        let mut config = Appearance {
            font_size: 0.,
            tab_size: 100,
            theme: "missing".into(),
            ..Default::default()
        };
        config.normalize();
        assert_eq!(config.font_size, 4.);
        assert_eq!(config.tab_size, 8);
        assert_eq!(config.theme, "winY Black");
        let old: Appearance = serde_json::from_str("{}").unwrap();
        assert!(old.show_tabs);
        assert_eq!(old, Appearance::default());
    }
}
