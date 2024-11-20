use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::disable_raw_mode;
use eyre::Result;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use super::cli::{self, Provider::Llm};

enum Menu {
    ChooseProvider,
    GoogleInfo,
    LlmInfo,
}

impl Menu {
    fn next(&self) -> Self {
        match self {
            Menu::ChooseProvider => Menu::GoogleInfo,
            Menu::GoogleInfo => Menu::LlmInfo,
            Menu::LlmInfo => Menu::ChooseProvider,
        }
    }

    fn previous(&self) -> Self {
        match self {
            Menu::ChooseProvider => Menu::LlmInfo,
            Menu::GoogleInfo => Menu::ChooseProvider,
            Menu::LlmInfo => Menu::GoogleInfo,
        }
    }
}

pub struct AppState {
    current_screen: Menu,
    provider: cli::Provider,
    config_path: Option<PathBuf>,
    input_file: String,
    output_file: String,
    language_code: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_screen: Menu::ChooseProvider,
            provider: cli::Provider::GoogleTranslate {
                version: cli::ApiVersion::V2,
                credentials: String::new(),
            },
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

    terminal.draw(|f| {
        let provider_list = List::new(vec![
            ListItem::new("Google Translate"),
            ListItem::new("LLM"),
        ])
        .block(Block::default().title("Providers").borders(Borders::ALL));

        let config_list = List::new(vec![ListItem::new("None")])
            .block(Block::default().title("Config Path").borders(Borders::ALL));

        let input_list = List::new(vec![ListItem::new("None")])
            .block(Block::default().title("Input File").borders(Borders::ALL));

        let output_list = List::new(vec![ListItem::new("None")])
            .block(Block::default().title("Output File").borders(Borders::ALL));

        let language_list = List::new(vec![ListItem::new("None")]).block(
            Block::default()
                .title("Language Code")
                .borders(Borders::ALL),
        );

        f.render_widget(provider_list, chunks[0]);
        f.render_widget(config_list, chunks[1]);
        f.render_widget(input_list, chunks[2]);
        f.render_widget(output_list, chunks[3]);
        f.render_widget(language_list, chunks[4]);
    })?;

    Ok(())
}

pub fn handle_event(key: event::KeyEvent, app_state: Arc<Mutex<AppState>>) -> Result<()> {
    match key {
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            let selected = &app_state.lock().unwrap().current_screen;
            app_state.lock().unwrap().current_screen = selected.previous();
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            let selected = &app_state.lock().unwrap().current_screen;
            app_state.lock().unwrap().current_screen = selected.next();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;

            match app_state.lock().unwrap().current_screen {
                Menu::ChooseProvider => todo!(),
                Menu::GoogleInfo => todo!(),
                Menu::LlmInfo => todo!(),
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
