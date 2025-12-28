use crate::types::Bookmark;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct BookmarksPanel<'a> {
    bookmarks: &'a [Bookmark],
    selected_idx: Option<usize>,
    focused: bool,
}

impl<'a> BookmarksPanel<'a> {
    pub fn new(bookmarks: &'a [Bookmark], selected_idx: Option<usize>, focused: bool) -> Self {
        Self {
            bookmarks,
            selected_idx,
            focused,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .title("Bookmarks")
            .borders(Borders::ALL)
            .border_style(border_style);

        if self.bookmarks.is_empty() {
            // Show empty state
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "[No bookmarks]",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press 'm' to add",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            let paragraph = ratatui::widgets::Paragraph::new(empty_text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(paragraph, area);
        } else {
            // Show bookmark list
            let items: Vec<ListItem> = self
                .bookmarks
                .iter()
                .map(|bookmark| {
                    let content = format!(
                        "Ch {} | Line {} | {}",
                        bookmark.chapter_idx + 1,
                        bookmark.line + 1,
                        bookmark.label
                    );
                    ListItem::new(Line::from(content))
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            // Create list state
            let mut list_state = ListState::default();
            list_state.select(self.selected_idx);

            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}
