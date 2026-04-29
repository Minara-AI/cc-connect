//! Left pane (in the claude-left/chat-right layout): the embedded `claude`
//! PTY for the active [`RoomTab`], rendered through tui-term over the
//! tab's vt100 screen.

use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Block, Borders},
    Frame,
};
use tui_term::widget::PseudoTerminal;

use crate::tabs::RoomTab;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, tab: &RoomTab, focused: bool) {
    let border_style = if focused {
        theme::border_focused()
    } else {
        theme::border_unfocused()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::BORDER_TYPE)
        .border_style(border_style)
        .title(Span::styled(
            format!(" 🤖 claude · {} ", tab.topic_short()),
            theme::pane_title(),
        ));
    let widget = PseudoTerminal::new(tab.vt_parser.screen()).block(block);
    frame.render_widget(widget, area);
}
