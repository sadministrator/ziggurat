use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::disable_raw_mode;
use eyre::Result;
use tui::style::{Color, Modifier, Style};
use tui::text::Span;
use tui::widgets::ListState;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use super::cli;

enum MenuOption {
    Providers,
    Config,
    Input,
    Output,
    Language,
}

enum Provider {
    Google,
    Llm,
}

impl MenuOption {
    fn next(&self) -> Self {
        match self {
            Self::Providers => Self::Config,
            Self::Config => Self::Input,
            Self::Input => Self::Output,
            Self::Output => Self::Language,
            Self::Language => Self::Providers,
        }
    }

    fn previous(&self) -> Self {
        match self {
            Self::Providers => Self::Language,
            Self::Config => Self::Providers,
            Self::Input => Self::Config,
            Self::Output => Self::Input,
            Self::Language => Self::Output,
        }
    }

    fn index(&self) -> usize {
        match self {
            Self::Providers => 0,
            Self::Config => 1,
            Self::Input => 2,
            Self::Output => 3,
            Self::Language => 4,
        }
    }
}

pub struct AppState {
    pub selected: MenuOption,
    provider: Option<cli::Provider>,
    config_path: Option<PathBuf>,
    input_file: String,
    output_file: String,
    language_code: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected: MenuOption::Providers,
            provider: None,
            config_path: None,
            input_file: String::new(),
            output_file: String::new(),
            language_code: String::new(),
        }
    }
}

pub fn render_app_state<B>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<()>
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(terminal.size().unwrap());

    let normal_style = Style::default();
    let highlighted_style = Style::default()
        .bg(Color::Yellow)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let state = &app_state.lock().unwrap();

    terminal.draw(|f| {
        let provider_items = vec![
            ListItem::new(Span::raw("Google Translate")),
            ListItem::new(Span::raw("LLM")),
        ];
        let provider_list = List::new(provider_items)
            .block(Block::default().title("Providers").borders(Borders::ALL))
            .highlight_style(if state.selected.index() == 0 {
                highlighted_style
            } else {
                normal_style
            })
            .highlight_symbol("> ");

        let mut provider_state = ListState::default();
        provider_state.select(Some(state.selected.index()));

        let config_list = List::new(vec![list_item("None", state.selected.index() == 1)])
            .block(Block::default().title("Config Path").borders(Borders::ALL));

        let input_list = List::new(vec![list_item("None", state.selected.index() == 2)])
            .block(Block::default().title("Input Path").borders(Borders::ALL));

        let output_list = List::new(vec![list_item("None", state.selected.index() == 3)])
            .block(Block::default().title("Output Path").borders(Borders::ALL));

        let language_list = List::new(vec![ListItem::new("None")])
            .block(
                Block::default()
                    .title("Language Code")
                    .borders(Borders::ALL),
            )
            .style(if state.selected.index() == 4 {
                highlighted_style
            } else {
                normal_style
            });

        f.render_stateful_widget(provider_list, chunks[0], &mut provider_state);
        f.render_widget(config_list, chunks[1]);
        f.render_widget(input_list, chunks[2]);
        f.render_widget(output_list, chunks[3]);
        f.render_widget(language_list, chunks[4]);
    })?;

    Ok(())
}

pub fn handle_event(key: KeyEvent, app_state: Arc<Mutex<AppState>>) -> Result<()> {
    let mut state = app_state.lock().unwrap();

    match key {
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            state.selected = state.selected.previous();
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.selected = state.selected.next();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;

            match state.selected {
                MenuOption::Providers => todo!(),
                MenuOption::Config => todo!(),
                MenuOption::Input => todo!(),
                MenuOption::Output => todo!(),
                MenuOption::Language => todo!(),
            };
        }
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        } => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                disable_raw_mode()?;
                std::process::exit(0)
            }
        }
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        } => {
            disable_raw_mode()?;
            std::process::exit(0)
        }
        _ => {}
    }

    Ok(())
}

fn list_item(content: &str, is_selected: bool) -> ListItem {
    let style = if is_selected {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    ListItem::new(Span::styled(content, style))
}
