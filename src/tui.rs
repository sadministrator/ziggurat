use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crossterm::event::{self, KeyCode, KeyEvent};
use eyre::Result;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use crate::cli::{self, Provider};

enum Screens {
    ChooseProvider,
    GoogleInfo,
    LlmInfo,
}

pub struct AppState {
    current_screen: Screens,
    provider: cli::Provider,
    config_path: Option<PathBuf>,
    input_file: String,
    output_file: String,
    language_code: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_screen: Screens::ChooseProvider,
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
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(terminal.size()?);
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

    terminal.draw(|f| {
        f.render_widget(provider_list, chunks[0]);
    })?;

    terminal.draw(|f| {
        f.render_widget(config_list, chunks[1]);
    })?;

    terminal.draw(|f| {
        f.render_widget(input_list, chunks[2]);
    })?;

    terminal.draw(|f| {
        f.render_widget(output_list, chunks[3]);
    })?;

    terminal.draw(|f| {
        f.render_widget(language_list, chunks[4]);
    })?;

    Ok(())
}

pub fn handle_event(key: event::KeyEvent, app_state: Arc<Mutex<AppState>>) -> Result<()> {
    match key {
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            app_state.lock().unwrap().current_screen = todo!();
        }
        KeyEvent {
            code: KeyCode::Char('l'),
            ..
        } => {
            app_state.lock().unwrap().provider = Provider::Llm {
                endpoint: String::new(),
                api_key: String::new(),
            };
        }
        KeyEvent {
            code: KeyCode::Char('c'),
            ..
        } => {
            let mut config_path = String::new();
            io::stdin().read_line(&mut config_path)?;

            app_state.lock().unwrap().config_path = Some(PathBuf::from(config_path.trim()));
        }
        KeyEvent {
            code: KeyCode::Char('i'),
            ..
        } => {
            let mut input_file = String::new();
            io::stdin().read_line(&mut input_file)?;

            app_state.lock().unwrap().input_file = input_file.trim().to_string();
        }
        KeyEvent {
            code: KeyCode::Char('o'),
            ..
        } => {
            let mut output_file = String::new();
            io::stdin().read_line(&mut output_file)?;

            app_state.lock().unwrap().output_file = output_file.trim().to_string();
        }
        KeyEvent {
            code: KeyCode::Char('a'),
            ..
        } => {
            let mut language_code = String::new();
            io::stdin().read_line(&mut language_code)?;

            app_state.lock().unwrap().language_code = language_code.trim().to_string();
        }
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        } => {}
        _ => {}
    }

    Ok(())
}
