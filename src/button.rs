use tui::{text::{Line, Span}, style::{Style, Color, Modifier}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub shortcut: String,
    pub text: String,
}

impl Button {
    pub fn new (mut text: String, shortcut: Option<String>) -> Self {
        if let Some(short) = shortcut {
            return Button {
                shortcut: short,
                text,
            };
        }
        let s = text.remove(0).to_string();
        Button {
            shortcut: s,
            text,
        }
        
    }
}

pub fn footer_button<'a>(button: Button) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {}", button.shortcut), Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", button.text), Style::default().fg(Color::White).bg(Color::LightBlue)),
    ])
}