use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use ratatui::style::Color;

pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme_name = detect_theme();

        CodeHighlighter {
            syntax_set,
            theme_set,
            theme_name,
        }
    }

    /// Highlight a code block with the given language
    pub fn highlight_code(&self, code: &str, language: Option<&str>) -> Vec<(String, Color)> {
        let mut result = Vec::new();

        // Get syntax reference
        let syntax = if let Some(lang) = language {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        // Get theme
        let theme = self.theme_set.themes.get(&self.theme_name)
            .or_else(|| self.theme_set.themes.get("base16-ocean.dark"))
            .expect("Failed to load theme");

        let mut highlighter = HighlightLines::new(syntax, theme);

        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_else(|_| vec![(Style::default(), line)]);

            for (style, text) in ranges {
                let color = syntect_to_ratatui_color(style.foreground);
                result.push((text.to_string(), color));
            }
        }

        result
    }
}

/// Detect terminal theme (light or dark)
fn detect_theme() -> String {
    use termbg::Theme;

    match termbg::theme(std::time::Duration::from_millis(100)) {
        Ok(Theme::Light) => "base16-ocean.light".to_string(),
        Ok(Theme::Dark) | Err(_) => "base16-ocean.dark".to_string(),
    }
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}
