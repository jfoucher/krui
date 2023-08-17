use tui::text::Line;
use tui::prelude::*;

pub fn parse<'a>(text: &'a str) -> Vec<Line> {
    let mut ret: Vec<Line> = vec![];
    for line in text.split("\n") {
        if line.starts_with("# ") {
            let mut l = Line::from(line.replacen("# ", "", 1)).alignment(Alignment::Center);
            l.patch_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
            ret.push(l);
        } else if line.starts_with("## ") {
            ret.push(
                Line::from(line.replacen("## ", "", 1)).alignment(Alignment::Center)
            )
        } else if line.starts_with("- ") {
            ret.push(
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Green)),
                    Span::from(line.replacen("- ", "", 1)),
                ]).alignment(Alignment::Left)
            )
        } else {
            ret.push(
                Line::from(line)
            )
        }
    }

    ret
}