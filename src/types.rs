// ModernTerm - Types Module
// Based on tterm's proven architecture with Makepad integration

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

// Re-export makepad types we'll use
pub use makepad_widgets::*;

/// View mode for terminal display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewMode {
    /// Single terminal view (with tabs)
    Single,
    /// Grid view with dynamic layout
    Grid { 
        rows: usize, 
        cols: usize,
        // Custom cell sizes: col_ratios[i] = width ratio for column i
        // row_ratios[i] = height ratio for row i
        col_ratios: Vec<f32>,
        row_ratios: Vec<f32>,
    },
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Single
    }
}

/// Direction for splitting panels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Recursive panel content structure (proven in tterm)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelContent {
    /// Single terminal panel
    Terminal(u64), // terminal_id
    /// Split panel containing two sub-panels
    Split {
        direction: SplitDirection,
        first: Box<PanelContent>,
        second: Box<PanelContent>,
        ratio: f32, // 0.1 to 0.9 range for safety
    },
}

/// Terminal tab representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub current_directory: Option<String>,
    pub process_name: Option<String>,
    pub input_buffer: String,  // Track current input line
    pub command_history: Vec<String>,  // Command history
    pub output_lines: Vec<String>,  // Terminal output lines
}

/// Size representation for layout calculations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
        }
    }
}

impl From<DVec2> for Size {
    fn from(vec: DVec2) -> Self {
        Self {
            width: vec.x as f32,
            height: vec.y as f32,
        }
    }
}

/// Korean IME state for individual terminals
#[derive(Debug, Clone, Default)]
pub struct KoreanInputState {
    pub composing: bool,
    pub chosung: Option<char>,    // 초성 (consonant)
    pub jungsung: Option<char>,   // 중성 (vowel)
    pub jongsung: Option<char>,   // 종성 (final consonant)
    pub composed_char: Option<char>, // 완성된 문자
}

impl KoreanInputState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn clear(&mut self) {
        self.composing = false;
        self.chosung = None;
        self.jungsung = None;
        self.jongsung = None;
        self.composed_char = None;
    }
    
    pub fn is_empty(&self) -> bool {
        !self.composing && self.chosung.is_none() && self.jungsung.is_none() && self.jongsung.is_none()
    }
}

// Import the real terminal backend
pub use crate::terminal::{TerminalBackend, BackendSettings};

// Use alacritty's event types
pub use alacritty_terminal::event::Event as PtyEvent;

/// Main application state - based on tterm's proven architecture
pub struct AppState {
    // Tab management (HashMap + Vec combination for efficiency + order)
    pub tabs: HashMap<u64, TerminalTab>,
    pub tab_order: Vec<u64>, // Maintain tab order
    pub active_tab_id: u64,
    pub next_tab_id: u64,
    
    // Terminal backends
    pub terminals: HashMap<u64, TerminalBackend>, // All terminal backends
    pub next_terminal_id: u64,
    pub tab_layouts: HashMap<u64, PanelContent>, // Layout for each tab
    
    // View state
    pub view_mode: ViewMode,
    pub focused_terminal: Option<u64>,
    
    // Broadcasting system (proven in tterm)
    pub broadcast_mode: bool,
    pub selected_terminals: HashSet<u64>, // Terminals to broadcast to
    
    // Korean IME support (per-terminal independent states)
    pub korean_input_states: HashMap<u64, KoreanInputState>,
    // Terminal scroll tracking: follow at bottom and manual offsets
    pub follow_output: HashMap<u64, bool>,
    pub scroll_offset: HashMap<u64, i32>,
    

    
    // Initialization state
    pub initialized: bool,
    pub pty_thread_started: bool,
    
    // Makepad context - remove for now
    // pub cx: Cx,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            tab_order: Vec::new(),
            active_tab_id: 0,
            next_tab_id: 1,
            terminals: HashMap::new(),
            next_terminal_id: 1,
            tab_layouts: HashMap::new(),
            view_mode: ViewMode::Single,
            focused_terminal: None,
            broadcast_mode: false,
            selected_terminals: HashSet::new(),
            korean_input_states: HashMap::new(),
            follow_output: HashMap::new(),
            scroll_offset: HashMap::new(),
            initialized: false,
            pty_thread_started: false,
        }
    }
    
    /// Create a new terminal instance
    pub fn create_terminal(&mut self, signal: SignalToUI) -> u64 {
        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;
        
        // Create terminal backend with alacritty_terminal
        match TerminalBackend::new(
            terminal_id,
            signal,
            BackendSettings::default(),
        ) {
            Ok(terminal_backend) => {
                self.terminals.insert(terminal_id, terminal_backend);
                self.korean_input_states.insert(terminal_id, KoreanInputState::new());
                ::log::info!("Created terminal backend {}", terminal_id);
            }
            Err(e) => {
                ::log::error!("Failed to create terminal backend: {}", e);
            }
        }
        
        terminal_id
    }
    
    /// Get the currently active terminal ID
    pub fn get_active_terminal(&self) -> Option<u64> {
        self.focused_terminal.or_else(|| {
            // Fallback to first terminal in active tab
            if let Some(layout) = self.tab_layouts.get(&self.active_tab_id) {
                self.find_first_terminal_in_layout(layout)
            } else {
                None
            }
        })
    }
    
    /// Recursively find the first terminal in a layout
    pub fn find_first_terminal_in_layout(&self, layout: &PanelContent) -> Option<u64> {
        match layout {
            PanelContent::Terminal(id) => Some(*id),
            PanelContent::Split { first, .. } => {
                self.find_first_terminal_in_layout(first)
            }
        }
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub appearance: AppearanceConfig,
    pub behavior: BehaviorConfig,
    pub keyboard: KeyboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub font_family: String,
    pub font_size: f32,
    pub theme: String,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub scrollback_lines: usize,
    pub close_tab_on_exit: bool,
    pub confirm_quit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub new_tab: String,
    pub close_tab: String,
    pub split_vertical: String,
    pub split_horizontal: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig {
                font_family: "JetBrains Mono".to_string(),
                font_size: 14.0,
                theme: "dark".to_string(),
                opacity: 0.95,
            },
            behavior: BehaviorConfig {
                scrollback_lines: 10000,
                close_tab_on_exit: true,
                confirm_quit: true,
            },
            keyboard: KeyboardConfig {
                new_tab: "Ctrl+T".to_string(),
                close_tab: "Ctrl+W".to_string(),
                split_vertical: "Ctrl+Shift+V".to_string(),
                split_horizontal: "Ctrl+Shift+H".to_string(),
            },
        }
    }
}
