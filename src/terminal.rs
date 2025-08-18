// ModernTerm - Terminal Backend
// Based on tterm's successful alacritty_terminal integration

use crate::types::{PtyEvent, Size};
use alacritty_terminal::event::{Event, EventListener, Notify, OnResize, WindowSize};
use makepad_widgets::SignalToUI;
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{self, cell::Cell, test::TermSize, Term, TermMode};
use alacritty_terminal::{tty, Grid};
use alacritty_terminal::index::{Point};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Result;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

/// Terminal backend settings (based on tterm)
#[derive(Debug, Clone)]
pub struct BackendSettings {
    pub shell: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl Default for BackendSettings {
    fn default() -> Self {
        let mut env = HashMap::new();
        
        // Ensure UTF-8 locale is properly set for Korean/Unicode support
        env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
        env.insert("LC_ALL".to_string(), "en_US.UTF-8".to_string());
        env.insert("LC_CTYPE".to_string(), "en_US.UTF-8".to_string());
        
        let default_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        
        Self {
            shell: default_shell,
            args: vec![],
            working_directory: None,
            env,
        }
    }
}

/// Terminal size representation
#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cell_width: u16,
    pub cell_height: u16,
    num_cols: u16,
    num_lines: u16,
    layout_size: Size,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cell_width: 6,  // Default monospace width (matching app.rs calculation)
            cell_height: 12, // Default monospace height (matching app.rs calculation)
            num_cols: 120,   // Larger default size to ensure content is visible
            num_lines: 30,   // Larger default size to ensure content is visible
            layout_size: Size::default(),
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines as usize
    }

    fn columns(&self) -> usize {
        self.num_cols as usize
    }

    fn last_column(&self) -> Column {
        Column(self.num_cols as usize - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.num_lines as i32 - 1)
    }
}

impl From<TerminalSize> for WindowSize {
    fn from(size: TerminalSize) -> Self {
        Self {
            num_lines: size.num_lines,
            num_cols: size.num_cols,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
        }
    }
}

/// Backend command for terminal operations
#[derive(Debug, Clone)]
pub enum BackendCommand {
    Write(Vec<u8>),
    Scroll(i32),
    Resize(Size, Size),
}

/// Terminal backend (based on tterm's successful implementation)
pub struct TerminalBackend {
    id: u64,
    pty_id: u32,
    term: Arc<FairMutex<Term<EventProxy>>>,
    size: TerminalSize,
    notifier: Notifier,
    last_content: RenderableContent,
}

impl TerminalBackend {
    pub fn new(
        id: u64,
        signal: SignalToUI,
        settings: BackendSettings,
    ) -> Result<Self> {
        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(settings.shell, settings.args)),
            working_directory: settings.working_directory,
            env: settings.env,
            ..tty::Options::default()
        };
        let config = term::Config::default();
        let terminal_size = TerminalSize::default();
        let pty = tty::new(&pty_config, terminal_size.into(), id)?;
        let pty_id = pty.child().id();
        let (event_sender, event_receiver) = mpsc::channel();
        let event_proxy = EventProxy(event_sender);
        let mut term = Term::new(config, &terminal_size, event_proxy.clone());
        let initial_content = RenderableContent {
            grid: term.grid().clone(),
            selectable_range: None,
            terminal_mode: *term.mode(),
            terminal_size,
            cursor: term.grid_mut().cursor_cell().clone(),
            cursor_line: 0,
            cursor_col: 0,
        };
        let term = Arc::new(FairMutex::new(term));
        let pty_event_loop = EventLoop::new(term.clone(), event_proxy, pty, false, false)?;
        let notifier = Notifier(pty_event_loop.channel());
        let _pty_event_loop_thread = pty_event_loop.spawn();
        let _pty_event_subscription = std::thread::Builder::new()
            .name(format!("pty_event_subscription_{}", id))
            .spawn(move || loop {
                if let Ok(event) = event_receiver.recv() {
                    println!(">>>>>>>>>>>>Received event: {:?}", event);
                    // 바로 UI에 신호 전송 - 불필요한 중간 채널 제거
                    signal.set();
                    if let Event::Exit = event {
                        break;
                    }
                }
            })?;

        Ok(Self {
            id,
            pty_id,
            term: term.clone(),
            size: terminal_size,
            notifier,
            last_content: initial_content,
        })
    }

    pub fn process_command(&mut self, cmd: BackendCommand) {
        let term = self.term.clone();
        let mut term = term.lock();
        match cmd {
            BackendCommand::Write(input) => {
                self.write(input);
                term.scroll_display(Scroll::Bottom);
            },
            BackendCommand::Scroll(delta) => {
                self.scroll(&mut term, delta);
            },
            BackendCommand::Resize(layout_size, font_size) => {
                self.resize(&mut term, layout_size, font_size);
            },
        };
    }

    pub fn sync(&mut self) -> &RenderableContent {
        let term = self.term.clone();
        let mut terminal = term.lock();
        let selectable_range = match &terminal.selection {
            Some(s) => s.to_range(&terminal),
            None => None,
        };

        let cursor = terminal.grid_mut().cursor_cell().clone();
        let grid_ref = terminal.grid();
        let point: Point = grid_ref.cursor.point;
        self.last_content.grid = grid_ref.clone();
        self.last_content.selectable_range = selectable_range;
        self.last_content.cursor = cursor.clone();
        self.last_content.terminal_mode = *terminal.mode();
        self.last_content.terminal_size = self.size;
        self.last_content.cursor_line = point.line.0 as usize;
        self.last_content.cursor_col = point.column.0 as usize;
        &self.last_content
    }

    pub fn last_content(&self) -> &RenderableContent {
        &self.last_content
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn pty_id(&self) -> u32 {
        self.pty_id
    }

    fn resize(&mut self, terminal: &mut Term<EventProxy>, layout_size: Size, font_size: Size) {
        if layout_size == self.size.layout_size
            && font_size.width as u16 == self.size.cell_width
            && font_size.height as u16 == self.size.cell_height
        {
            return;
        }

        let lines = (layout_size.height / font_size.height.floor()) as u16;
        let cols = (layout_size.width / font_size.width.floor()) as u16;
        if lines > 0 && cols > 0 {
            self.size = TerminalSize {
                layout_size,
                cell_height: font_size.height as u16,
                cell_width: font_size.width as u16,
                num_lines: lines,
                num_cols: cols,
            };

            self.notifier.on_resize(self.size.into());
            terminal.resize(TermSize::new(
                self.size.num_cols as usize,
                self.size.num_lines as usize,
            ));
        }
    }

    fn write<I: Into<Cow<'static, [u8]>>>(&self, input: I) {
        self.notifier.notify(input);
    }

    fn scroll(&mut self, terminal: &mut Term<EventProxy>, delta_value: i32) {
        if delta_value != 0 {
            let scroll = Scroll::Delta(delta_value);
            if terminal
                .mode()
                .contains(TermMode::ALTERNATE_SCROLL | TermMode::ALT_SCREEN)
            {
                let line_cmd = if delta_value > 0 { b'A' } else { b'B' };
                let mut content = vec![];

                for _ in 0..delta_value.abs() {
                    content.push(0x1b);
                    content.push(b'O');
                    content.push(line_cmd);
                }

                self.notifier.notify(content);
            } else {
                terminal.grid_mut().scroll_display(scroll);
            }
        }
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);
    }
}

/// Renderable terminal content
#[derive(Clone)]
pub struct RenderableContent {
    pub grid: Grid<Cell>,
    pub selectable_range: Option<SelectionRange>,
    pub cursor: Cell,
    pub terminal_mode: TermMode,
    pub terminal_size: TerminalSize,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl Default for RenderableContent {
    fn default() -> Self {
        Self {
            grid: Grid::new(0, 0, 0),
            selectable_range: None,
            cursor: Cell::default(),
            terminal_mode: TermMode::empty(),
            terminal_size: TerminalSize::default(),
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

/// Event proxy for alacritty
#[derive(Clone)]
pub struct EventProxy(mpsc::Sender<Event>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event.clone());
    }
}


