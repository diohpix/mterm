// ModernTerm - Application Core
// Based on tterm's proven architecture with Makepad integration

use crate::types::*;
use makepad_widgets::*;
use makepad_widgets::event::ScrollEvent;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;

live_design!{
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    
    // Use system default font for now
    // D2_CODING_FONT = {
    //     path: dep("crate://self/assets/fonts/D2Coding.ttf") 
    // }
        
    App = {{App}} {
        ui: <Root>{
            main_window = <Window>{
                body = <View>{
                    flow: Down,
                    spacing: 0,
                    align: {
                        x: 0.0,
                        y: 0.0
                    },
                    
                    // Tab bar
                    tab_bar = <View> {
                        height: 32,
                        flow: Right,
                        spacing: 4,
                        padding: {left: 8, right: 8, top: 4, bottom: 4},
                        show_bg: true,
                        draw_bg: {
                            color: #404040,
                        }
                        
                        // Tab buttons will be dynamically created
                        tab1 = <Button> {
                            text: "● Terminal 1",
                            margin: {right: 4},
                            padding: {left: 12, right: 12, top: 6, bottom: 6},
                            draw_text: {
                                color: #ffffff,
                                text_style: {
                                    font_size: 12.0,
                                }
                            },
                            draw_bg: {
                                color: #707070,
                            }
                        }
                        
                        // New tab button
                        new_tab_btn = <Button> {
                            text: "+",
                            width: 32,
                            height: 28,
                            margin: {left: 8},
                            draw_text: {
                                color: #ffffff,
                                text_style: {
                                    font_size: 16.0,
                                }
                            },
                            draw_bg: {
                                color: #505050,
                            }
                        }
                    }
                    
                    // Main terminal area  
                    terminal_area = <View> {
                        height: 600,
                        width: Fill,
                        scroll_bars: <ScrollBars> { 
                            show_scroll_y: true,
                        },
                        show_bg: true,
                        draw_bg: {
                            color: #1e1e1e,
                        }
                        
                        terminal_display = <Label> {
                            width: Fill,
                            height: Fit,
                            padding: {left: 12, right: 12, top: 12, bottom: 12},
                            draw_text: {
                                color: #c0c0c0,
                                text_style: {
                                    font_size: 10.0,
                                    line_spacing: 1.2
                                }
                            },
                            text: "",
                        }
                    }
                    
                    // Status bar
                    status_bar = <View> {
                        height: 24,
                        flow: Right,
                        spacing: 8,
                        padding: {left: 8, right: 8, top: 2, bottom: 2},
                        show_bg: true,
                        draw_bg: {
                            color: #3c3c3c,
                        }
                        
                        status_text = <Label> {
                            draw_text: {
                                color: #ffffff,
                                text_style: {
                                    font_size: 11.0,
                                }
                            },
                            text: "Ready | Single Mode | Terminal 1",
                        }
                    }
                }
            }
        }
    }
}
app_main!(App); 
#[derive(Live, LiveHook)]
pub struct App {
    #[live] ui: WidgetRef,
    #[rust] state: AppState,
    #[rust] pty_signal: SignalToUI,  // PTY 이벤트용 신호 (공식 API)
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) { 
        crate::makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App{
    fn match_event(&mut self, cx: &mut Cx, event: &Event) {
        // Handle startup first
        if !self.state.is_initialized() {
            println!("🚀 ModernTerm started! - BASIC PRINTLN (from match_event)");
            self.handle_startup_internal(cx);
        }
        
        // Then handle the actual event
        match event {
            Event::Startup => {
                println!("📱 Received Event::Startup");
            }
            Event::Draw(draw_event) => {
                // Remove spam - only log occasionally
                static mut DRAW_COUNT: u32 = 0;
                static mut WINDOW_RESIZE_DONE: bool = false;
                unsafe {
                    DRAW_COUNT += 1;
                    if DRAW_COUNT % 100 == 0 {
                        println!("🎨 Received Draw event #{}", DRAW_COUNT);
                    }
                    
                    // Skip window resize for now to avoid blocking startup
                    if !WINDOW_RESIZE_DONE && DRAW_COUNT > 5 {
                        println!("🪟 UI is ready, but skipping window resize for now");
                        WINDOW_RESIZE_DONE = true;
                    }
                }
                self.handle_draw(cx, draw_event);
            }


            Event::WindowGeomChange(_) => {
                // Handle window resize to adjust terminal size
                self.handle_window_resize(cx);
            }
            _ => {}
        }
        
        // Call default match_event implementation for other UI handling
        // WidgetMatchEvent::match_event(&mut self.ui, cx, event);
    }

    fn handle_startup(&mut self, cx:&mut Cx){
        // This method might not be called in newer Makepad versions
        println!("🚀 ModernTerm started! - BASIC PRINTLN");
        ::log::info!("🚀 ModernTerm started!");
        
        // Initialize SignalToUI for PTY events (공식 API)
        self.pty_signal = SignalToUI::new();
        println!("📡 PTY SignalToUI initialized");
        
        // Initialize state
        self.state = AppState::new();
        ::log::info!("📋 AppState initialized with {} terminals", self.state.terminals.len());
        
        // Create the first tab automatically
        println!("📄 Creating initial tab - BASIC PRINTLN");
        Self::create_new_tab(&mut self.state, self.pty_signal.clone());
        println!("📄 Created initial tab - BASIC PRINTLN");
        ::log::info!("📄 Created initial tab automatically");
        
        // Calculate and set initial terminal size immediately
        self.handle_window_resize(cx);
        ::log::info!("📐 Initial terminal size calculated");
        
        // Update terminal display with the new tab  
        self.refresh_terminal_content(cx);
        
        // Process any initial PTY events immediately so the prompt shows without extra input
        self.handle_pty_events(cx);
        self.refresh_terminal_content(cx);
        ::log::info!("🔄 Initial PTY read completed");
        
        self.ui.redraw(cx);
        
        ::log::info!("✅ ModernTerm initialization complete - Ready for use!");

        // Note: Using event-driven updates instead of timer polling
    }
        
    fn handle_actions(&mut self, cx: &mut Cx, actions:&Actions){
        // Always check for PTY events first - this replaces timer polling
        self.handle_pty_events(cx);
        
        // Handle button/widget actions
        // For now, we'll use keyboard shortcuts for tab management
        // Full button handling can be implemented later when we understand Makepad's action API better
        ::log::debug!("Actions received: {:?}", actions.len());
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Always check for PTY events at the start of each event cycle
        self.handle_pty_events(cx);
        
        // Handle system events first
        match event {
            Event::WindowCloseRequested(_) => {
                self.quit_application(cx);
            }
            Event::KeyDown(key_event) => {
                self.handle_key_down(cx, key_event);
            }
            Event::TextInput(text_event) => {
                self.handle_text_input(cx, text_event);
            }
            Event::Scroll(scroll_event) => {
                self.handle_scroll(cx, scroll_event);
            }
            Event::WindowGeomChange(_) => {
                // Handle window resize to adjust terminal size
                self.handle_window_resize(cx);
            }
            // Timer events are handled in custom match_event implementation above
            _ => {}
        }
        
        // Let MatchEvent handle its events
        self.match_event(cx, event);
        
        // Let the UI handle the event
        self.ui.handle_event(cx, event, &mut Scope::empty());
        
        // Check PTY events again after all processing - ensures immediate updates
        self.handle_pty_events(cx);
    }
}

impl App {
    fn handle_startup_internal(&mut self, cx: &mut Cx) {
        println!("🚀 ModernTerm started! - BASIC PRINTLN");
        ::log::info!("🚀 ModernTerm started!");
        
        // Initialize SignalToUI for PTY events (공식 API)
        self.pty_signal = SignalToUI::new();
        println!("📡 PTY SignalToUI initialized");
        
        // Initialize state
        self.state = AppState::new();
        ::log::info!("📋 AppState initialized with {} terminals", self.state.terminals.len());
        
        // Create the first tab automatically
        println!("📄 Creating initial tab - BASIC PRINTLN");
        Self::create_new_tab(&mut self.state, self.pty_signal.clone());
        println!("📄 Created initial tab - BASIC PRINTLN");
        ::log::info!("📄 Created initial tab automatically");
        
        // Skip complex operations during startup - defer to timer
        ::log::info!("📐 Skipping window resize during startup - will be handled after first draw");
        
        // Now we can safely update terminal display and handle PTY events
        self.refresh_terminal_content(cx);
        self.handle_pty_events(cx);
        ::log::info!("🖥️ Terminal display refreshed and PTY events processed");
        
        // Note: Using event-driven updates instead of timer polling
        ::log::info!("💓 Event-driven PTY processing enabled - no timer polling");
        
        self.ui.redraw(cx);
        ::log::info!("✅ ModernTerm initialization complete - Ready for use!");
        
        // Mark as initialized
        self.state.mark_initialized();
    }
    


    pub fn new(cx: &mut Cx) -> Self {
        // Initialize the application state
        let mut state = AppState::new();
        let pty_signal = SignalToUI::new();
        
        // Create the first tab with a terminal
        Self::create_new_tab(&mut state, pty_signal.clone());
        
        Self {
            ui: WidgetRef::default(),
            state,
            pty_signal,
        }
    }
    
    /// Create a new tab with a terminal (based on tterm's TabManager)
    fn create_new_tab(state: &mut AppState, signal: SignalToUI) {
        let tab_id = state.next_tab_id;
        state.next_tab_id += 1;
        
        // Create the terminal for this tab
        let terminal_id = state.create_terminal(signal);
        
        // Create more descriptive tab title
        let shell_name = std::env::var("SHELL")
            .unwrap_or_else(|_| "/bin/bash".to_string())
            .split('/')
            .last()
            .unwrap_or("bash")
            .to_string();
        
        // Create the tab
        let tab = TerminalTab {
            id: tab_id,
            title: format!("{} {}", shell_name, tab_id),
            current_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            process_name: Some(shell_name),
            input_buffer: String::new(),
            command_history: Vec::new(),
            output_lines: Vec::new(),
        };
        
        // Set up the layout for this tab (single terminal)
        let layout = PanelContent::Terminal(terminal_id);
        
        let tab_title = tab.title.clone();
        
        // Add to state
        state.tabs.insert(tab_id, tab);
        state.tab_order.push(tab_id);
        state.tab_layouts.insert(tab_id, layout);
        state.active_tab_id = tab_id;
        state.focused_terminal = Some(terminal_id);
        
        ::log::info!("📄 Created new tab '{}' (ID: {}) with terminal {}", 
                    tab_title, tab_id, terminal_id);
    }
    
    
    
    /// Update status bar with current information
    fn update_status_bar(&mut self, cx: &mut Cx) {
        let tab_count = self.state.tabs.len();
        let current_tab_index = self.state.tab_order.iter()
            .position(|&id| id == self.state.active_tab_id)
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let view_mode_text = match &self.state.view_mode {
            ViewMode::Single => "Single Mode",
            ViewMode::Grid { rows, cols, .. } => &format!("Grid {}x{}", rows, cols),
        };
        
        let focused_terminal_info = if let Some(terminal_id) = self.state.focused_terminal {
            format!("Terminal {}", terminal_id)
        } else {
            "No Terminal".to_string()
        };
        
        let status_text = format!(
            "Ready | {} | Tab {}/{} | {} | Ctrl+T:New Ctrl+W:Close Ctrl+Q:Quit",
            view_mode_text,
            current_tab_index,
            tab_count,
            focused_terminal_info
        );
        
        let status_label = self.ui.label(id!(status_text));
        status_label.set_text(cx, &status_text);
    }
     
   
    /// Extract text content from alacritty terminal grid (optimized)
    fn extract_grid_text(content: &crate::terminal::RenderableContent) -> String {
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::index::{Line, Column};
        
        let grid = &content.grid;
        let num_lines = grid.screen_lines();
        let num_cols = grid.columns();
        
        // Use precise cursor position from backend renderable content
        let cursor_line_idx: usize = content.cursor_line.min(num_lines.saturating_sub(1));
        let cursor_col_idx: usize = content.cursor_col.min(num_cols.saturating_sub(1));

        // Pre-allocate string capacity for better performance
        let mut display_text = String::with_capacity(num_lines * num_cols);
        
        // Extract text from each line of the grid
        for line_index in 0..num_lines {
            let line = Line::from(line_index);
            let mut line_chars: Vec<char> = Vec::with_capacity(num_cols);
            
            // Extract characters from each column
            for col_index in 0..num_cols {
                let column = Column::from(col_index);
                let cell = &grid[line][column];
                line_chars.push(cell.c);
            }
            
            // If this is the cursor line, insert a full block cursor at the exact column.
            if line_index == cursor_line_idx {
                if cursor_col_idx < num_cols {
                    line_chars[cursor_col_idx] = '\u{2588}';
                } else if num_cols > 0 {
                    line_chars[num_cols - 1] = '\u{2588}';
                }
                let line_text: String = line_chars.into_iter().collect();
                display_text.push_str(&line_text);
            } else {
                // Convert to string and trim trailing spaces
                let line_text: String = line_chars.into_iter().collect();
                let trimmed = line_text.trim_end();
                if !trimmed.is_empty() || line_index == 0 {
                    display_text.push_str(trimmed);
                }
            }
            
            // Add newline except for the last line
            if line_index < num_lines - 1 {
                display_text.push('\n');
            }
        }
        
        // Fallback if empty
        if display_text.trim().is_empty() {
            display_text = "ModernTerm Terminal - Real PTY Mode\nType any command and press Enter!\n\n$ ".to_string();
        }

        display_text
    }



    /// Handle PTY signals using SignalToUI (non-blocking)
    fn handle_pty_events(&mut self, cx: &mut Cx) {
        // Check for SignalToUI signals from PTY thread (non-blocking)
        if self.pty_signal.check_and_clear() {
            println!("🎨 SignalToUI signal received! Refreshing terminal...");
            // Signal received - refresh terminal content and redraw UI
            self.refresh_terminal_content(cx);
            cx.redraw_all();
            println!("🎨 SignalToUI processing completed!");
        }
    }
    
    /// Check if a layout contains a specific terminal
    fn contains_terminal_in_layout(&self, layout: &PanelContent, terminal_id: u64) -> bool {
        match layout {
            PanelContent::Terminal(id) => *id == terminal_id,
            PanelContent::Split { first, second, .. } => {
                self.contains_terminal_in_layout(first, terminal_id) ||
                self.contains_terminal_in_layout(second, terminal_id)
            }
        }
    }
    
    /// Handle text input events
    fn handle_text_input(&mut self, cx: &mut Cx, text_event: &TextInputEvent) {
        if let Some(terminal_id) = self.state.focused_terminal {
            // Update the input buffer for current tab
            let active_tab_id = self.state.active_tab_id;
            
            // Reduce logging for performance
        if text_event.input.len() > 1 {
            ::log::debug!("🔤 Processing text input: {} chars", text_event.input.len());
        }
            
        for ch in text_event.input.chars() {
                match ch {
                    '\r' | '\n' => {
                        ::log::info!("⏎ Enter key detected!");
                        // Handle Enter key - execute command
                        let _command = if let Some(tab) = self.state.tabs.get_mut(&active_tab_id) {
                            let cmd = tab.input_buffer.trim().to_string();
                            if !cmd.is_empty() {
                                // Add to command history
                                tab.command_history.push(cmd.clone());
                                // Clear input buffer
                                tab.input_buffer.clear();
                                Some(cmd)
                            } else {
                                // Even for empty commands, add a blank line to simulate Enter
                                tab.output_lines.push("".to_string());
                                tab.input_buffer.clear();
                                None
                            }
                        } else {
                            None
                        };
                        
                        // Note: Real PTY will handle command execution
                        // No need for simulation anymore!
                        
                        // Send to actual terminal backend too
                        if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                            backend.process_command(crate::terminal::BackendCommand::Write(b"\r".to_vec()));
                        }
                        
                        // Update display immediately after Enter
                        self.refresh_terminal_content(cx);
                    }
                    '\x08' | '\x7f' => {
                        // Handle Backspace
                        if let Some(tab) = self.state.tabs.get_mut(&active_tab_id) {
                            tab.input_buffer.pop();
                        }
                        // Update display for backspace
                        self.refresh_terminal_content(cx);
                    }
                    _ if ch.is_control() => {
                        // Skip other control characters for input buffer
                    }
                    _ => {
                        // Regular character - add to input buffer
                        if let Some(tab) = self.state.tabs.get_mut(&active_tab_id) {
                            tab.input_buffer.push(ch);
                        }
                        
                        // Send to terminal backend
                        if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                            let mut char_bytes = [0u8; 4];
                            let char_str = ch.encode_utf8(&mut char_bytes);
                            backend.process_command(crate::terminal::BackendCommand::Write(char_str.as_bytes().to_vec()));
                        }
                        
                        // Reduce refresh frequency for regular typing
                        // Only refresh every few characters for performance
                        static mut CHAR_COUNT: u32 = 0;
                        unsafe {
                            CHAR_COUNT += 1;
                            if CHAR_COUNT % 3 == 0 { // Update every 3rd character
                                self.refresh_terminal_content(cx);
                            }
                        }
                    }
                }
            }
            
            ::log::info!("Processed text input: {:?}", text_event.input);
            
            // Update display after processing input
            self.refresh_terminal_content(cx);
        } else {
            ::log::info!("No focused terminal for text input: {:?}", text_event.input);
        }
    }
    
    /// Handle scroll events for terminal history
    fn handle_scroll(&mut self, cx: &mut Cx, scroll_event: &ScrollEvent) {
        if let Some(terminal_id) = self.state.focused_terminal {
            if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                // Convert scroll to terminal scroll delta
                // Negative delta = scroll up (show older content)
                // Positive delta = scroll down (show newer content)
                let delta = -(scroll_event.scroll.y * 3.0) as i32; // Multiply for sensitivity and invert
                
                ::log::info!("📜 Scroll event: delta={}, terminal_id={}", delta, terminal_id);
                
                // Send scroll command to terminal backend
                backend.process_command(crate::terminal::BackendCommand::Scroll(delta));
                
                // User scrolled: disable follow at bottom until they return
                self.state.follow_output.insert(terminal_id, false);
                // Update display immediately after scroll
                self.refresh_terminal_content(cx);
            }
        }
    }
    
    /// Refresh terminal content display (optimized)
    fn refresh_terminal_content(&mut self, cx: &mut Cx) {
        // Get the current terminal content from the backend
        if let Some(terminal_id) = self.state.focused_terminal {
            // Split borrows: get content first, then release backend borrow
            let content_opt = if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                Some(backend.sync().clone())
            } else { None };
            if let Some(content) = content_opt.as_ref() {
                // Check if we're in alternative screen mode (like top, vi, etc.)
                let is_alt_screen = content.terminal_mode.contains(TermMode::ALT_SCREEN);
                
                // Extract text content from terminal grid
                let terminal_text = Self::extract_grid_text(content);
                
                // Debug: Log alternative screen mode (keep this for now)
                if is_alt_screen {
                    println!("🔍 Alternative screen mode detected");
                }
                
                // Update the terminal display Label with actual content
                let terminal_display_label = self.ui.label(id!(terminal_display));
                
                // Debug: Check if label is found (simplified)
                if terminal_display_label.is_empty() {
                    println!("❌ Error: terminal_display label not found!");
                }
                
                terminal_display_label.set_text(cx, &terminal_text);

                // Force immediate and strong UI refresh
                if is_alt_screen {
                    // Alternative screen mode (like top, vi) needs aggressive redraw
                    println!("🔥 ALT_SCREEN: Performing aggressive redraw");
                    cx.redraw_all();
                    self.ui.redraw(cx);
                    
                    // Force redraw of all UI components for alternative screen apps
                    let root_view = self.ui.view(id!(root));
                    root_view.redraw(cx);
                    
                    let terminal_area = self.ui.view(id!(terminal_area));
                    terminal_area.redraw(cx);
                    
                    // Additional aggressive refresh
                    terminal_display_label.redraw(cx);
                } else {
                    // Normal mode - standard redraw
                    cx.redraw_all();
                    self.ui.redraw(cx);
                }

                // Content will automatically fit using height: Fit in terminal_display

                // Auto-scroll to cursor position when enabled
                if let Some(terminal_id) = self.state.focused_terminal {
                    let follow = self.state.follow_output.get(&terminal_id).copied().unwrap_or(true);
                    if follow {
                        let area_view = self.ui.view(id!(terminal_area));
                        
                        // Calculate cursor position in pixels
                        let line_height = 12.0; // Font line height
                        let cursor_y_pixels = (content.cursor_line as f64) * line_height;
                        
                        // Get view dimensions
                        let view_rect = area_view.area().rect(cx);
                        let view_height = view_rect.size.y;
                        
                        // Only scroll if cursor is beyond visible area
                        if cursor_y_pixels > view_height {
                            // Scroll so cursor is near bottom of visible area
                            let target_scroll_y = cursor_y_pixels - view_height + (line_height * 2.0);
                            area_view.set_scroll_pos(cx, DVec2 { x: 0.0, y: target_scroll_y.max(0.0) });
                        }
                    }
                }
            }
        }
    }
    
    /// Handle window resize to adjust terminal size
    fn handle_window_resize(&mut self, cx: &mut Cx) {
        ::log::info!("🔧 Window resized, calculating new terminal size");
        
        // Get the terminal area widget dimensions
        let terminal_area = self.ui.view(id!(terminal_area));
        let rect = terminal_area.area().rect(cx);
        
        ::log::info!("📐 Terminal area size: {}x{}", rect.size.x, rect.size.y);
        
        // Calculate available space (minus padding)
        let padding = 24.0f64; // 12px padding on each side
        let available_width = (rect.size.x - padding).max(100.0f64);
        let available_height = (rect.size.y - padding).max(100.0f64);
        
        // Font metrics based on the current settings (10px font size)
        let font_width = 6.0f64; // Approximate monospace character width for 10px
        let font_height = 12.0f64; // 10px * 1.2 line spacing
        
        // Calculate new terminal dimensions
        let new_cols = (available_width / font_width).floor() as u16;
        let new_rows = (available_height / font_height).floor() as u16;
        
        ::log::info!("🎯 Calculated terminal size: {}x{} (was default 80x24)", new_cols, new_rows);
        
        // Resize all active terminals
        for (&terminal_id, backend) in self.state.terminals.iter_mut() {
            let new_size = crate::types::Size {
                width: available_width as f32,
                height: available_height as f32,
            };
            
            let font_size = crate::types::Size {
                width: font_width as f32,
                height: font_height as f32,
            };
            
            backend.process_command(crate::terminal::BackendCommand::Resize(new_size, font_size));
            ::log::info!("📏 Resized terminal {} to {}x{}", terminal_id, new_cols, new_rows);
        }
        
        // Update display after resize
        self.refresh_terminal_content(cx);
    }
    
    /// Handle keyboard input
    fn handle_key_down(&mut self, cx: &mut Cx, key_event: &KeyEvent) {
        let modifiers = &key_event.modifiers;
        
        ::log::info!("Key down event: {:?} with modifiers: {:?}", key_event.key_code, modifiers);
        
        // Handle application shortcuts (based on tterm's InputHandler)
        match key_event.key_code {
            KeyCode::ReturnKey => {
                // Handle Enter key explicitly
                ::log::info!("⏎ Enter key detected in KeyDown!");
                if let Some(terminal_id) = self.state.focused_terminal {
                    let active_tab_id = self.state.active_tab_id;
                    
                                         let _command = if let Some(tab) = self.state.tabs.get_mut(&active_tab_id) {
                        let cmd = tab.input_buffer.trim().to_string();
                        if !cmd.is_empty() {
                            // Add to command history
                            tab.command_history.push(cmd.clone());
                            // Clear input buffer
                            tab.input_buffer.clear();
                            Some(cmd)
                        } else {
                            // Even for empty commands, add a blank line to simulate Enter
                            tab.output_lines.push("".to_string());
                            tab.input_buffer.clear();
                            None
                        }
                    } else {
                        None
                    };
                    
                    // Note: Real PTY will handle command execution
                    // No need for simulation anymore!
                    
                    // Send to actual terminal backend too
                    if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                        backend.process_command(crate::terminal::BackendCommand::Write(b"\r".to_vec()));
                    }
                    
                    // Update display immediately after Enter
                    self.refresh_terminal_content(cx);
                }
            }
            KeyCode::KeyT if modifiers.control => {
                // Ctrl+T: New tab
                Self::create_new_tab(&mut self.state, self.pty_signal.clone());
                self.refresh_terminal_content(cx);
                ::log::info!("New tab created via Ctrl+T");
            }
            KeyCode::KeyW if modifiers.control => {
                // Ctrl+W: Close tab
                self.close_current_tab(cx);
            }
            KeyCode::KeyS if modifiers.control => {
                // Ctrl+S: Toggle grid view
                self.toggle_grid_view(cx);
            }
            KeyCode::KeyQ if modifiers.control => {
                // Ctrl+Q: Quit application
                self.quit_application(cx);
            }
            KeyCode::KeyR if modifiers.control => {
                // Ctrl+R: Refresh terminal display
                self.refresh_terminal_content(cx);
                ::log::info!("Terminal display refreshed via Ctrl+R");
            }
            KeyCode::KeyL if modifiers.control && modifiers.shift => {
                // Ctrl+Shift+L: Clear terminal (send clear command)
                if let Some(terminal_id) = self.state.focused_terminal {
                    if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                        backend.process_command(crate::terminal::BackendCommand::Write(b"clear\r".to_vec()));
                        self.refresh_terminal_content(cx);
                        ::log::info!("Sent clear command via Ctrl+Shift+L");
                    }
                }
            }
            KeyCode::PageUp => {
                // Page Up: Scroll up in terminal history
                if let Some(terminal_id) = self.state.focused_terminal {
                    if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                        let delta = if modifiers.shift { -10 } else { -5 }; // Shift+PageUp scrolls more
                        backend.process_command(crate::terminal::BackendCommand::Scroll(delta));
                        self.refresh_terminal_content(cx);
                        ::log::info!("Page Up scroll: delta={}", delta);
                    }
                }
            }
            KeyCode::PageDown => {
                // Page Down: Scroll down in terminal history
                if let Some(terminal_id) = self.state.focused_terminal {
                    if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                        let delta = if modifiers.shift { 10 } else { 5 }; // Shift+PageDown scrolls more
                        backend.process_command(crate::terminal::BackendCommand::Scroll(delta));
                        self.refresh_terminal_content(cx);
                        ::log::info!("Page Down scroll: delta={}", delta);
                    }
                }
            }
            code if modifiers.control => {
                // Ctrl+1-9: Switch to tab by number
                if let Some(tab_number) = code.to_digit() {
                    self.switch_to_tab_by_number(cx, tab_number as usize);
                }
            }
            _ => {
                // Forward other keys to the focused terminal
                self.forward_key_to_terminal(cx, key_event);
                // Typing implies user wants to follow the latest output again
                if let Some(terminal_id) = self.state.focused_terminal {
                    self.state.follow_output.insert(terminal_id, true);
                }
            }
        }
    }
    
    /// Close the current tab
    fn close_current_tab(&mut self, cx: &mut Cx) {
        let active_tab_id = self.state.active_tab_id;
        
        if self.state.tabs.len() <= 1 {
            // Last tab - quit application
            self.quit_application(cx);
            return;
        }
        
        // Remove tab from order
        if let Some(pos) = self.state.tab_order.iter().position(|&id| id == active_tab_id) {
            self.state.tab_order.remove(pos);
            
            // Switch to next tab (or previous if this was the last)
            let new_active_index = if pos >= self.state.tab_order.len() {
                self.state.tab_order.len().saturating_sub(1)
            } else {
                pos
            };
            
            if let Some(&new_active_id) = self.state.tab_order.get(new_active_index) {
                self.state.active_tab_id = new_active_id;
                
                // Set focus to first terminal in the new active tab
                if let Some(layout) = self.state.tab_layouts.get(&new_active_id) {
                    self.state.focused_terminal = self.state.find_first_terminal_in_layout(layout);
                }
            }
        }
        
        // Clean up the closed tab
        self.state.tabs.remove(&active_tab_id);
        if let Some(layout) = self.state.tab_layouts.remove(&active_tab_id) {
            self.cleanup_terminals_in_layout(&layout);
        }
        
        self.ui.redraw(cx);
        ::log::info!("Closed tab {}", active_tab_id);
    }
    
    /// Toggle between single and grid view
    fn toggle_grid_view(&mut self, cx: &mut Cx) {
        match &self.state.view_mode {
            ViewMode::Single => {
                if self.state.tabs.len() > 1 {
                    // Switch to grid view
                    let tab_count = self.state.tabs.len();
                    let (rows, cols) = Self::calculate_optimal_grid_size(tab_count);
                    
                    self.state.view_mode = ViewMode::Grid {
                        rows,
                        cols,
                        col_ratios: vec![1.0 / cols as f32; cols],
                        row_ratios: vec![1.0 / rows as f32; rows],
                    };
                    
                    ::log::info!("Switched to grid view ({}x{})", rows, cols);
                } else {
                    ::log::info!("Cannot switch to grid view with only one tab");
                }
            }
            ViewMode::Grid { .. } => {
                // Switch to single view
                self.state.view_mode = ViewMode::Single;
                ::log::info!("Switched to single view");
            }
        }
        
        self.ui.redraw(cx);
    }
    
    /// Calculate optimal grid size for given number of tabs
    fn calculate_optimal_grid_size(tab_count: usize) -> (usize, usize) {
        match tab_count {
            0..=1 => (1, 1),
            2 => (1, 2),
            3 => (2, 2), // Special case: 2x1 + 1x2 layout
            4 => (2, 2),
            5..=6 => (2, 3),
            7..=9 => (3, 3),
            10..=12 => (3, 4),
            13..=16 => (4, 4),
            _ => {
                let sqrt = (tab_count as f32).sqrt().ceil() as usize;
                (sqrt, sqrt)
            }
        }
    }
    
    /// Switch to tab by number (1-based)
    fn switch_to_tab_by_number(&mut self, cx: &mut Cx, number: usize) {
        if number > 0 && number <= self.state.tab_order.len() {
            let tab_id = self.state.tab_order[number - 1];
            self.state.active_tab_id = tab_id;
            
            // Set focus to first terminal in the tab
            if let Some(layout) = self.state.tab_layouts.get(&tab_id) {
                self.state.focused_terminal = self.state.find_first_terminal_in_layout(layout);
            }
            
            self.ui.redraw(cx);
            ::log::info!("Switched to tab {} (ID: {})", number, tab_id);
        }
    }
    
    /// Cleanup terminals in a layout recursively
    fn cleanup_terminals_in_layout(&mut self, layout: &PanelContent) {
        match layout {
            PanelContent::Terminal(terminal_id) => {
                self.state.terminals.remove(terminal_id);
                self.state.korean_input_states.remove(terminal_id);
                ::log::debug!("Cleaned up terminal {}", terminal_id);
            }
            PanelContent::Split { first, second, .. } => {
                self.cleanup_terminals_in_layout(first);
                self.cleanup_terminals_in_layout(second);
            }
        }
    }
    
    /// Forward key events to the focused terminal
    fn forward_key_to_terminal(&mut self, cx: &mut Cx, key_event: &KeyEvent) {
        if let Some(terminal_id) = self.state.focused_terminal {
            // First get the bytes to send
            if let Some(bytes) = self.key_to_bytes(key_event) {
                // Then get mutable reference to backend
                if let Some(backend) = self.state.terminals.get_mut(&terminal_id) {
                    backend.process_command(crate::terminal::BackendCommand::Write(bytes));
                    ::log::debug!("Forwarded key to terminal {}: {:?}", terminal_id, key_event.key_code);
                    
                    // Update display after sending input
                    self.refresh_terminal_content(cx);
                }
            }
        }
    }
    
    /// Convert key events to bytes for terminal input (based on tterm)
    fn key_to_bytes(&self, key_event: &KeyEvent) -> Option<Vec<u8>> {
        let modifiers = &key_event.modifiers;
        
        match key_event.key_code {
            // Special keys that need specific handling
            // Note: Return/Enter is typically handled by TextInput event in Makepad
            KeyCode::Tab => Some(b"\t".to_vec()),
            KeyCode::Escape => Some(b"\x1b".to_vec()),
            KeyCode::Backspace => Some(b"\x7f".to_vec()),
            KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
            
            // Arrow keys
            KeyCode::ArrowUp => Some(b"\x1b[A".to_vec()),
            KeyCode::ArrowDown => Some(b"\x1b[B".to_vec()),
            KeyCode::ArrowRight => Some(b"\x1b[C".to_vec()),
            KeyCode::ArrowLeft => Some(b"\x1b[D".to_vec()),
            
            // Home/End
            KeyCode::Home => Some(b"\x1b[H".to_vec()),
            KeyCode::End => Some(b"\x1b[F".to_vec()),
            
            // Page Up/Down
            KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
            KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
            
            // Function keys
            KeyCode::F1 => Some(b"\x1bOP".to_vec()),
            KeyCode::F2 => Some(b"\x1bOQ".to_vec()),
            KeyCode::F3 => Some(b"\x1bOR".to_vec()),
            KeyCode::F4 => Some(b"\x1bOS".to_vec()),
            KeyCode::F5 => Some(b"\x1b[15~".to_vec()),
            KeyCode::F6 => Some(b"\x1b[17~".to_vec()),
            KeyCode::F7 => Some(b"\x1b[18~".to_vec()),
            KeyCode::F8 => Some(b"\x1b[19~".to_vec()),
            KeyCode::F9 => Some(b"\x1b[20~".to_vec()),
            KeyCode::F10 => Some(b"\x1b[21~".to_vec()),
            KeyCode::F11 => Some(b"\x1b[23~".to_vec()),
            KeyCode::F12 => Some(b"\x1b[24~".to_vec()),
            
            // Control combinations that don't interfere with app shortcuts
            KeyCode::KeyA if modifiers.control => Some(b"\x01".to_vec()), // Ctrl+A (Home)
            KeyCode::KeyB if modifiers.control => Some(b"\x02".to_vec()), // Ctrl+B (Left)
            KeyCode::KeyC if modifiers.control => Some(b"\x03".to_vec()), // Ctrl+C (SIGINT)
            KeyCode::KeyD if modifiers.control => Some(b"\x04".to_vec()), // Ctrl+D (EOF)
            KeyCode::KeyE if modifiers.control => Some(b"\x05".to_vec()), // Ctrl+E (End)
            KeyCode::KeyF if modifiers.control => Some(b"\x06".to_vec()), // Ctrl+F (Right)
            KeyCode::KeyG if modifiers.control => Some(b"\x07".to_vec()), // Ctrl+G (Bell)
            KeyCode::KeyH if modifiers.control => Some(b"\x08".to_vec()), // Ctrl+H (Backspace)
            KeyCode::KeyI if modifiers.control => Some(b"\x09".to_vec()), // Ctrl+I (Tab)
            KeyCode::KeyJ if modifiers.control => Some(b"\x0a".to_vec()), // Ctrl+J (LF)
            KeyCode::KeyK if modifiers.control => Some(b"\x0b".to_vec()), // Ctrl+K (VT)
            KeyCode::KeyL if modifiers.control => Some(b"\x0c".to_vec()), // Ctrl+L (Clear)
            KeyCode::KeyM if modifiers.control => Some(b"\x0d".to_vec()), // Ctrl+M (CR)
            KeyCode::KeyN if modifiers.control => Some(b"\x0e".to_vec()), // Ctrl+N (Down)
            KeyCode::KeyO if modifiers.control => Some(b"\x0f".to_vec()), // Ctrl+O 
            KeyCode::KeyP if modifiers.control => Some(b"\x10".to_vec()), // Ctrl+P (Up)
            KeyCode::KeyR if modifiers.control => Some(b"\x12".to_vec()), // Ctrl+R (Search)
            KeyCode::KeyU if modifiers.control => Some(b"\x15".to_vec()), // Ctrl+U (Kill line)
            KeyCode::KeyV if modifiers.control => Some(b"\x16".to_vec()), // Ctrl+V 
            KeyCode::KeyX if modifiers.control => Some(b"\x18".to_vec()), // Ctrl+X
            KeyCode::KeyY if modifiers.control => Some(b"\x19".to_vec()), // Ctrl+Y
            KeyCode::KeyZ if modifiers.control => Some(b"\x1a".to_vec()), // Ctrl+Z (SIGTSTP)
            
            _ => None, // Most printable keys handled by text input event
        }
    }
    

    
    /// Simulate command execution for better user experience
    
    
    /// Quit the application
    fn quit_application(&mut self, _cx: &mut Cx) {
        ::log::info!("Quitting ModernTerm");
        std::process::exit(0);
    }
}



impl Default for App {
    fn default() -> Self {
        let mut state = AppState::new();
        let pty_signal = SignalToUI::new();
        
        // Create the first tab automatically
        Self::create_new_tab(&mut state, pty_signal.clone());
        
        Self {
            ui: WidgetRef::default(),
            state,
            pty_signal,
        }
    }
}

// Helper trait for key code to digit conversion
trait KeyCodeExt {
    fn to_digit(&self) -> Option<u32>;
}

impl KeyCodeExt for KeyCode {
    fn to_digit(&self) -> Option<u32> {
        match self {
            KeyCode::Key1 => Some(1),
            KeyCode::Key2 => Some(2),
            KeyCode::Key3 => Some(3),
            KeyCode::Key4 => Some(4),
            KeyCode::Key5 => Some(5),
            KeyCode::Key6 => Some(6),
            KeyCode::Key7 => Some(7),
            KeyCode::Key8 => Some(8),
            KeyCode::Key9 => Some(9),
            _ => None,
        }
    }
}