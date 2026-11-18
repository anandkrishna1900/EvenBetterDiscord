//! Theme schema, validation, and preview generation.
//!
//! Themes are data, not code: only a fixed set of fields is accepted, colours
//! must be hex literals, and free-form text is screened before it reaches CSS.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const THEME_FILE: &str = "theme.json";
pub const SCHEMA_VERSION: u32 = 1;

/// A validated theme document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub colors: ThemeColors,
    #[serde(default)]
    pub typography: Typography,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeColors {
    pub background: String,
    pub foreground: String,
    pub accent: String,
    #[serde(default)]
    pub muted: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Typography {
    pub font_family: Option<String>,
    pub base_size_px: Option<u16>,
}

/// Everything a UI needs to render a theme preview without parsing files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreview {
    pub css: String,
    pub swatches: BTreeMap<String, String>,
}

impl ThemeManifest {
    pub fn parse(contents: &str) -> Result<Self, ThemeError> {
        let theme: Self = serde_json::from_str(contents).map_err(ThemeError::Malformed)?;
        theme.validate()?;
        Ok(theme)
    }

    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ThemeError::UnsupportedVersion(self.schema_version));
        }
        validate_identifier("id", &self.id)?;
        if self.name.trim().is_empty() || self.name.len() > 80 {
            return Err(ThemeError::InvalidField("name"));
        }
        if let Some(description) = &self.description {
            if description.len() > 500 {
                return Err(ThemeError::InvalidField("description"));
            }
        }

        for (field, color) in [
            ("colors.background", &self.colors.background),
            ("colors.foreground", &self.colors.foreground),
            ("colors.accent", &self.colors.accent),
        ] {
            validate_color(field, color)?;
        }
        if let Some(muted) = &self.colors.muted {
            validate_color("colors.muted", muted)?;
        }

        if let Some(font) = &self.typography.font_family {
            if font.is_empty()
                || font.len() > 120
                || font
                    .chars()
                    .any(|character| matches!(character, ';' | '{' | '}' | '<' | '>' | '\n' | '\r'))
            {
                return Err(ThemeError::InvalidField("typography.font_family"));
            }
        }
        if let Some(size) = self.typography.base_size_px {
            if !(8..=32).contains(&size) {
                return Err(ThemeError::InvalidField("typography.base_size_px"));
            }
        }
        Ok(())
    }

    /// Produces CSS custom properties and a swatch list for preview UIs.
    pub fn preview(&self) -> Result<ThemePreview, ThemeError> {
        self.validate()?;

        let mut swatches = BTreeMap::new();
        swatches.insert("background".to_owned(), self.colors.background.clone());
        swatches.insert("foreground".to_owned(), self.colors.foreground.clone());
        swatches.insert("accent".to_owned(), self.colors.accent.clone());
        if let Some(muted) = &self.colors.muted {
            swatches.insert("muted".to_owned(), muted.clone());
        }

        let mut css = format!(
            ":root {{\n  --ebd-bg: {};\n  --ebd-fg: {};\n  --ebd-accent: {};\n",
            self.colors.background, self.colors.foreground, self.colors.accent
        );
        if let Some(muted) = &self.colors.muted {
            css.push_str(&format!("  --ebd-muted: {muted};\n"));
        }
        if let Some(font) = &self.typography.font_family {
            css.push_str(&format!("  --ebd-font: {font};\n"));
        }
        if let Some(size) = self.typography.base_size_px {
            css.push_str(&format!("  --ebd-font-size: {size}px;\n"));
        }
        css.push_str("}\n");

        Ok(ThemePreview { css, swatches })
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ThemeError> {
    let valid = (3..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(ThemeError::InvalidField(field))
    }
}

fn validate_color(field: &'static str, color: &str) -> Result<(), ThemeError> {
    let hex = color.strip_prefix('#').unwrap_or_default();
    let valid = matches!(hex.len(), 3 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(ThemeError::InvalidField(field))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("invalid theme JSON: {0}")]
    Malformed(serde_json::Error),
    #[error("unsupported theme schema version {0}; expected {SCHEMA_VERSION}")]
    UnsupportedVersion(u32),
    #[error("invalid theme field: {0}")]
    InvalidField(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r##"{
        "schema_version": 1,
        "id": "midnight-blue",
        "name": "Midnight Blue",
        "colors": {
            "background": "#0b1020",
            "foreground": "#e6e9f0",
            "accent": "#4f8cff",
            "muted": "#8891a8"
        },
        "typography": { "font_family": "Inter, sans-serif", "base_size_px": 15 }
    }"##;

    #[test]
    fn parses_and_previews_a_valid_theme() {
        let theme = ThemeManifest::parse(VALID).expect("valid theme");
        let preview = theme.preview().expect("preview");

        assert!(preview.css.contains("--ebd-bg: #0b1020;"));
        assert!(preview.css.contains("--ebd-accent: #4f8cff;"));
        assert!(preview.css.contains("--ebd-font-size: 15px;"));
        assert_eq!(preview.swatches.len(), 4);
    }

    #[test]
    fn rejects_invalid_colors() {
        for color in ["0b1020", "#ggg", "#12345", "url(x)", "#12345678;"] {
            let json = VALID.replace("#0b1020", color);
            assert!(
                matches!(
                    ThemeManifest::parse(&json),
                    Err(ThemeError::InvalidField("colors.background"))
                ),
                "should reject {color}"
            );
        }
    }

    #[test]
    fn rejects_css_injection_in_font_family() {
        let json = VALID.replace("Inter, sans-serif", "Inter; color: red");
        assert!(matches!(
            ThemeManifest::parse(&json),
            Err(ThemeError::InvalidField("typography.font_family"))
        ));
    }

    #[test]
    fn rejects_unknown_fields_and_bad_schema_versions() {
        let extra = VALID.replace("\"colors\"", "\"surprise\": 1, \"colors\"");
        assert!(matches!(
            ThemeManifest::parse(&extra),
            Err(ThemeError::Malformed(_))
        ));

        let version = VALID.replace("\"schema_version\": 1", "\"schema_version\": 9");
        assert!(matches!(
            ThemeManifest::parse(&version),
            Err(ThemeError::UnsupportedVersion(9))
        ));
    }

    #[test]
    fn short_forms_are_allowed() {
        let json = r##"{
            "schema_version": 1,
            "id": "short",
            "name": "Short",
            "colors": { "background": "#fff", "foreground": "#000", "accent": "#abc" }
        }"##;
        let theme = ThemeManifest::parse(json).expect("short theme");
        assert!(theme.preview().is_ok());
    }
}
