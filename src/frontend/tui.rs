use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fmt, io};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    terminal::disable_raw_mode,
};
use eyre::Result;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use super::cli::Provider;

enum MenuOption {
    Providers,
    Config,
    Input,
    Output,
    Language,
}

enum ProviderOption {
    Add,
    Entry(ProviderEntry),
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

pub struct ProviderEntry {
    name: String,
    provider: Provider,
}

impl fmt::Display for ProviderEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self.provider {
                Provider::GoogleTranslate { .. } =>
                    format!("LLM Endpoint Credentials - {}", self.name),
                Provider::Llm { .. } => format!("Google Credentials - {}", self.name),
            }
        )
    }
}

pub struct AppState {
    selected_option: MenuOption,
    selected_provider: ProviderOption,
    confirmed_provider_idx: Option<usize>,
    providers: Vec<ProviderEntry>,
    config_path: Option<PathBuf>,
    input_file: String,
    output_file: String,
    language_code: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_option: MenuOption::Providers,
            selected_provider: ProviderOption::Add,
            confirmed_provider_idx: None,
            providers: vec![],
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

    let state = &app_state.lock().unwrap();

    terminal.draw(|f| {
        let mut provider_items = vec!["Add provider".to_string()];

        provider_items.extend(
            state
                .providers
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>(),
        );

        let provider_list = styled_list(
            "Providers",
            provider_items,
            state.selected_option.index() == 0,
        )
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

        let mut provider_state = ListState::default();
        provider_state.select(Some(state.selected_option.index()));

        let config_list = styled_list(
            "Config Path",
            vec!["None".to_string()],
            state.selected_option.index() == 1,
        );
        let input_list = styled_list(
            "Input Path",
            vec!["None".to_string()],
            state.selected_option.index() == 2,
        );
        let output_list = styled_list(
            "Output Path",
            vec!["None".to_string()],
            state.selected_option.index() == 3,
        );
        let language_list = styled_list(
            "Language Code",
            vec!["None".to_string()],
            state.selected_option.index() == 4,
        );

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
            state.selected_option = state.selected_option.previous();
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.selected_option = state.selected_option.next();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            match state.selected_option {
                MenuOption::Providers => {
                    match &state.selected_provider {
                        ProviderOption::Add => {
                            // go to menu with list: "Google Cloud Credentials", "OpenAI-compatible API Credentials"
                            todo!()
                        }
                        ProviderOption::Entry(ProviderEntry { name, provider }) => {
                            todo!()
                        }
                    };
                }
                MenuOption::Config => todo!(),
                MenuOption::Input => todo!(),
                MenuOption::Output => todo!(),
                MenuOption::Language => todo!(),
            };

            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;

            match state.selected_option {
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

fn styled_list(title: &str, items: Vec<String>, is_selected: bool) -> List {
    let style = if is_selected {
        Style::default()
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = items
        .into_iter()
        .map(|item| ListItem::new(Span::styled(item, Style::default())))
        .collect();

    List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(style)
}
