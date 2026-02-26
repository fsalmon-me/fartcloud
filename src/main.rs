//! FartCloud - Flappy Bird-style game with a farting cloud
//! Rust/Macroquad WASM game with Platform API leaderboard
//! Features: Altitude zones, directional farts, variable gravity
//! Modes: Anonymous (standalone) or Connected (platform API)

use macroquad::prelude::*;
use macroquad::audio::{load_sound, play_sound, set_sound_volume, Sound, PlaySoundParams};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// JAVASCRIPT INTEROP FOR MOBILE KEYBOARD
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod js_keyboard {
    extern "C" {
        pub fn js_show_mobile_keyboard();
        pub fn js_hide_mobile_keyboard();
        pub fn js_is_input_confirmed() -> i32;
        pub fn js_get_input_length() -> i32;
        pub fn js_get_input_value(ptr: *mut u8, max_len: i32) -> i32;
        pub fn js_reset_input();
    }
    
    pub fn show_keyboard() {
        unsafe { js_show_mobile_keyboard(); }
    }
    
    pub fn hide_keyboard() {
        unsafe { js_hide_mobile_keyboard(); }
    }
    
    pub fn is_confirmed() -> bool {
        unsafe { js_is_input_confirmed() != 0 }
    }
    
    pub fn get_input() -> String {
        unsafe {
            let len = js_get_input_length();
            if len <= 0 {
                return String::new();
            }
            let mut buffer = vec![0u8; len as usize];
            let actual_len = js_get_input_value(buffer.as_mut_ptr(), len);
            buffer.truncate(actual_len as usize);
            String::from_utf8(buffer).unwrap_or_default()
        }
    }
    
    pub fn reset() {
        unsafe { js_reset_input(); }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod js_keyboard {
    pub fn show_keyboard() {}
    pub fn hide_keyboard() {}
    pub fn is_confirmed() -> bool { false }
    pub fn get_input() -> String { String::new() }
    pub fn reset() {}
}

// ============================================================================
// JAVASCRIPT INTEROP FOR PLATFORM API
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod js_platform {
    extern "C" {
        // --- Score & Leaderboard ---
        pub fn js_submit_score(name_ptr: *const u8, name_len: i32, score: u32);
        pub fn js_submit_score_full(
            name_ptr: *const u8, name_len: i32,
            score: u32,
            diff_level: u32,
            combo_max: u32,
            fart_count: u32,
            duration_ms: u32,
            death_type_ptr: *const u8, death_type_len: i32,
            zone_ptr: *const u8, zone_len: i32,
        );
        pub fn js_fetch_leaderboard();
        pub fn js_is_leaderboard_ready() -> i32;
        pub fn js_get_leaderboard_count() -> i32;
        pub fn js_get_leaderboard_name(index: i32, ptr: *mut u8, max_len: i32) -> i32;
        pub fn js_get_leaderboard_score(index: i32) -> u32;
        pub fn js_reset_leaderboard();
        pub fn js_save_high_score(score: u32);
        pub fn js_get_high_score() -> u32;
        // --- Platform Config ---
        pub fn js_fetch_platform_config();
        pub fn js_is_platform_config_ready() -> i32;
        pub fn js_get_platform_config_json_length() -> i32;
        pub fn js_get_platform_config_json(ptr: *mut u8, max_len: i32) -> i32;
        // --- Auth ---
        pub fn js_is_authenticated() -> i32;
        pub fn js_is_auth_validated() -> i32;
        pub fn js_get_auth_username_length() -> i32;
        pub fn js_get_auth_username(ptr: *mut u8, max_len: i32) -> i32;
        pub fn js_is_platform_configured() -> i32;
        // --- Login (placeholder) ---
        pub fn js_platform_login(
            user_ptr: *const u8, user_len: i32,
            pass_ptr: *const u8, pass_len: i32,
        );
        pub fn js_is_login_complete() -> i32;
        pub fn js_is_login_success() -> i32;
    }
    
    pub fn submit_score(name: &str, score: u32) {
        unsafe {
            js_submit_score(name.as_ptr(), name.len() as i32, score);
        }
    }
    
    pub fn submit_score_full(
        name: &str, score: u32,
        diff_level: u32, combo_max: u32, fart_count: u32,
        duration_secs: f32, death_type: &str, zone: &str,
    ) {
        unsafe {
            let duration_ms = (duration_secs * 1000.0) as u32;
            js_submit_score_full(
                name.as_ptr(), name.len() as i32,
                score, diff_level, combo_max, fart_count, duration_ms,
                death_type.as_ptr(), death_type.len() as i32,
                zone.as_ptr(), zone.len() as i32,
            );
        }
    }
    
    pub fn fetch_leaderboard() {
        unsafe { js_fetch_leaderboard(); }
    }
    
    pub fn is_leaderboard_ready() -> bool {
        unsafe { js_is_leaderboard_ready() != 0 }
    }
    
    pub fn get_leaderboard_count() -> usize {
        unsafe { js_get_leaderboard_count() as usize }
    }
    
    pub fn get_leaderboard_entry(index: usize) -> (String, u32) {
        unsafe {
            let mut name_buf = vec![0u8; 64];
            let name_len = js_get_leaderboard_name(index as i32, name_buf.as_mut_ptr(), 64);
            name_buf.truncate(name_len as usize);
            let name = String::from_utf8(name_buf).unwrap_or_else(|_| "???".to_string());
            let score = js_get_leaderboard_score(index as i32);
            (name, score)
        }
    }
    
    pub fn reset_leaderboard() {
        unsafe { js_reset_leaderboard(); }
    }
    
    pub fn save_high_score(score: u32) {
        unsafe { js_save_high_score(score); }
    }
    
    pub fn get_high_score() -> u32 {
        unsafe { js_get_high_score() }
    }
    
    // --- Platform Config ---
    pub fn fetch_platform_config() {
        unsafe { js_fetch_platform_config(); }
    }
    
    pub fn is_platform_config_ready() -> bool {
        unsafe { js_is_platform_config_ready() != 0 }
    }
    
    pub fn get_platform_config_json() -> Option<String> {
        unsafe {
            let len = js_get_platform_config_json_length();
            if len <= 0 { return None; }
            let mut buf = vec![0u8; len as usize];
            let actual = js_get_platform_config_json(buf.as_mut_ptr(), len);
            buf.truncate(actual as usize);
            String::from_utf8(buf).ok()
        }
    }
    
    // --- Auth ---
    pub fn is_authenticated() -> bool {
        unsafe { js_is_authenticated() != 0 }
    }
    
    pub fn is_auth_validated() -> bool {
        unsafe { js_is_auth_validated() != 0 }
    }
    
    pub fn get_auth_username() -> String {
        unsafe {
            let len = js_get_auth_username_length();
            if len <= 0 { return "Anonyme".to_string(); }
            let mut buf = vec![0u8; len as usize];
            let actual = js_get_auth_username(buf.as_mut_ptr(), len);
            buf.truncate(actual as usize);
            String::from_utf8(buf).unwrap_or_else(|_| "Anonyme".to_string())
        }
    }
    
    pub fn is_platform_configured() -> bool {
        unsafe { js_is_platform_configured() != 0 }
    }
    
    // --- Login ---
    pub fn platform_login(username: &str, password: &str) {
        unsafe {
            js_platform_login(
                username.as_ptr(), username.len() as i32,
                password.as_ptr(), password.len() as i32,
            );
        }
    }
    
    pub fn is_login_complete() -> bool {
        unsafe { js_is_login_complete() != 0 }
    }
    
    pub fn is_login_success() -> bool {
        unsafe { js_is_login_success() != 0 }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod js_platform {
    pub fn submit_score(_name: &str, _score: u32) {}
    pub fn submit_score_full(
        _name: &str, _score: u32,
        _diff_level: u32, _combo_max: u32, _fart_count: u32,
        _duration_secs: f32, _death_type: &str, _zone: &str,
    ) {}
    pub fn fetch_leaderboard() {}
    pub fn is_leaderboard_ready() -> bool { true }
    pub fn get_leaderboard_count() -> usize { 0 }
    pub fn get_leaderboard_entry(_index: usize) -> (String, u32) { (String::new(), 0) }
    pub fn reset_leaderboard() {}
    pub fn save_high_score(_score: u32) {}
    pub fn get_high_score() -> u32 { 0 }
    pub fn fetch_platform_config() {}
    pub fn is_platform_config_ready() -> bool { true }
    pub fn get_platform_config_json() -> Option<String> { None }
    pub fn is_authenticated() -> bool { false }
    pub fn is_auth_validated() -> bool { true }
    pub fn get_auth_username() -> String { "Anonyme".to_string() }
    pub fn is_platform_configured() -> bool { false }
    pub fn platform_login(_username: &str, _password: &str) {}
    pub fn is_login_complete() -> bool { false }
    pub fn is_login_success() -> bool { false }
}

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Deserialize, Serialize, Clone)]
struct GameConfig {
    gravity_base: f32,
    fart_power_base: f32,
    player_size: f32,
    cloud_speed_initial: f32,
    cloud_speed_increment: f32,
    spawn_interval_initial: f32,
    spawn_interval_min: f32,
    difficulty_increase_every: u32,
    particle_count: u32,
    particle_lifetime: f32,
    shake_intensity: f32,
    shake_decay: f32,
    world_height: f32,
    camera_lerp: f32,
    // Audio settings
    #[serde(default = "default_master_volume")]
    master_volume: f32,
    #[serde(default = "default_sfx_volume")]
    sfx_volume: f32,
    // Difficulty system - degressive speed
    #[serde(default = "default_speed_transition_level")]
    speed_transition_level: u32,  // Level where speed growth slows down
    #[serde(default = "default_speed_slow_growth")]
    speed_slow_growth: f32,       // Growth rate after transition (e.g., 1% = 0.01)
    // Difficulty system - hybrid spawn
    #[serde(default = "default_cloud_density_factor")]
    cloud_density_factor: f32,    // Base multiplier for min clouds calculation
    #[serde(default = "default_cloud_level_increment")]
    cloud_level_increment: u32,   // +1 min cloud every N levels
    #[serde(default = "default_spawn_interval_decay")]
    spawn_interval_decay: f32,    // How fast spawn interval decreases per level
    // Future map system
    #[serde(default = "default_hardcore_factor")]
    hardcore_factor: f32,         // 1.0 = normal, 1.5 = hard, 2.0 = extreme
}

fn default_master_volume() -> f32 { 1.0 }
fn default_sfx_volume() -> f32 { 0.8 }
fn default_speed_transition_level() -> u32 { 10 }
fn default_speed_slow_growth() -> f32 { 0.01 }
fn default_cloud_density_factor() -> f32 { 0.008 }
fn default_cloud_level_increment() -> u32 { 3 }
fn default_spawn_interval_decay() -> f32 { 0.08 }
fn default_hardcore_factor() -> f32 { 1.0 }

// ============================================================================
// DIFFICULTY CALCULATION HELPERS
// ============================================================================

/// Calculate speed based on degressive formula:
/// - Before transition: linear growth (fast early game)
/// - After transition: percentage-based slow growth (+1% per level)
fn calculate_speed(level: u32, config: &GameConfig) -> f32 {
    let base = config.cloud_speed_initial;
    let increment = config.cloud_speed_increment;
    let transition = config.speed_transition_level;
    let slow_growth = config.speed_slow_growth;
    let hardcore = config.hardcore_factor;
    
    if level <= transition {
        // Linear growth phase: base + level * increment
        base + (level as f32 * increment * hardcore)
    } else {
        // Degressive phase: speed at transition + percentage growth
        let speed_at_transition = base + (transition as f32 * increment * hardcore);
        let levels_past = level - transition;
        // Each level adds slow_growth% of current speed
        speed_at_transition * (1.0 + slow_growth * hardcore).powi(levels_past as i32)
    }
}

/// Calculate minimum cloud count based on world height and difficulty
/// Base count from world_height * density_factor, +1 every N levels
fn calculate_min_clouds(level: u32, config: &GameConfig) -> u32 {
    let base_clouds = (config.world_height * config.cloud_density_factor) as u32;
    let level_bonus = level / config.cloud_level_increment;
    let hardcore_bonus = ((config.hardcore_factor - 1.0) * 2.0) as u32;
    base_clouds + level_bonus + hardcore_bonus
}

/// Calculate spawn interval minimum (affected by hardcore factor)
fn calculate_spawn_interval_min(config: &GameConfig) -> f32 {
    config.spawn_interval_min / config.hardcore_factor
}

// ============================================================================
// VIRTUAL RESOLUTION & SCALING (16:9 Letterboxing)
// ============================================================================

const VIRTUAL_WIDTH: f32 = 1067.0;  // 16:9 aspect ratio with height 600
const VIRTUAL_HEIGHT: f32 = 600.0;
const ASPECT_RATIO: f32 = VIRTUAL_WIDTH / VIRTUAL_HEIGHT; // 16:9 ≈ 1.778

/// Calculate letterbox parameters for 16:9 aspect ratio
/// Returns (scale, offset_x, offset_y, game_width, game_height)
fn letterbox_params() -> (f32, f32, f32, f32, f32) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let screen_aspect = screen_w / screen_h;
    
    let (game_w, game_h) = if screen_aspect > ASPECT_RATIO {
        // Screen is wider than 16:9 - pillarbox (black bars on sides)
        let h = screen_h;
        let w = h * ASPECT_RATIO;
        (w, h)
    } else {
        // Screen is taller than 16:9 - letterbox (black bars top/bottom)
        let w = screen_w;
        let h = w / ASPECT_RATIO;
        (w, h)
    };
    
    let offset_x = (screen_w - game_w) / 2.0;
    let offset_y = (screen_h - game_h) / 2.0;
    let scale = game_h / VIRTUAL_HEIGHT;
    
    (scale, offset_x, offset_y, game_w, game_h)
}

/// Get uniform scale factor - always 1.0 since we render at virtual resolution
fn scale() -> f32 {
    1.0
}

/// Get the game area width (always virtual width for render-to-texture)
fn game_width() -> f32 {
    VIRTUAL_WIDTH
}

/// Get the game area height (always virtual height for render-to-texture)
fn game_height() -> f32 {
    VIRTUAL_HEIGHT
}

/// Get the X offset for letterboxing (left black bar width)
fn game_offset_x() -> f32 {
    letterbox_params().1
}

/// Get the Y offset for letterboxing (top black bar height)
fn game_offset_y() -> f32 {
    letterbox_params().2
}

/// Draw letterbox bars (black bars around game area) ON TOP of the game content
/// This clips any content that renders outside the 16:9 game area
fn draw_letterbox_bars() {
    let (_, offset_x, offset_y, game_w, game_h) = letterbox_params();
    let sw = screen_width();
    let sh = screen_height();
    
    // Top bar
    if offset_y > 0.0 {
        draw_rectangle(0.0, 0.0, sw, offset_y, BLACK);
    }
    // Bottom bar
    if offset_y > 0.0 {
        draw_rectangle(0.0, offset_y + game_h, sw, sh - (offset_y + game_h), BLACK);
    }
    // Left bar
    if offset_x > 0.0 {
        draw_rectangle(0.0, 0.0, offset_x, sh, BLACK);
    }
    // Right bar
    if offset_x > 0.0 {
        draw_rectangle(offset_x + game_w, 0.0, sw - (offset_x + game_w), sh, BLACK);
    }
}

/// Convert screen coordinates to virtual game coordinates
/// Takes mouse/touch position in screen space and returns position in virtual 1067x600 space
fn screen_to_virtual(screen_x: f32, screen_y: f32) -> (f32, f32) {
    let (scale, offset_x, offset_y, _game_w, _game_h) = letterbox_params();
    
    // Remove offset and divide by scale to get virtual coords
    let vx = (screen_x - offset_x) / scale;
    let vy = (screen_y - offset_y) / scale;
    
    (vx, vy)
}

/// Get mouse position in virtual coordinates (1067x600 space)
fn virtual_mouse_position() -> (f32, f32) {
    let (sx, sy) = mouse_position();
    screen_to_virtual(sx, sy)
}

/// Scale a font size for current screen
fn scaled_font(base_size: f32) -> f32 {
    base_size * scale()
}

/// Scale a dimension (width, height, offset) for current screen
fn scaled(value: f32) -> f32 {
    value * scale()
}

/// Check if device is in portrait mode (needs rotation)
fn is_portrait() -> bool {
    screen_height() > screen_width()
}

/// Draw "rotate device" overlay - returns true if portrait (blocking)
fn draw_rotate_overlay() -> bool {
    if !is_portrait() {
        return false;
    }
    
    // Dark overlay
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.9));
    
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;
    
    // Rotating phone icon (ASCII art style)
    let icon_size = 80.0;
    draw_rectangle_lines(cx - icon_size / 4.0, cy - icon_size / 2.0, icon_size / 2.0, icon_size, 4.0, WHITE);
    
    // Rotation arrow
    let arrow_y = cy + icon_size / 2.0 + 20.0;
    draw_text("↻", cx - 20.0, arrow_y, 50.0, GOLD);
    
    // Message
    let msg1 = "Tourne ton appareil";
    let msg2 = "Mode paysage requis";
    let dim1 = measure_text(msg1, None, 28, 1.0);
    let dim2 = measure_text(msg2, None, 20, 1.0);
    draw_text(msg1, cx - dim1.width / 2.0, cy + icon_size + 60.0, 28.0, WHITE);
    draw_text(msg2, cx - dim2.width / 2.0, cy + icon_size + 90.0, 20.0, GRAY);
    
    true
}

// ============================================================================
// BUTTON SYSTEM
// ============================================================================

struct Button {
    text: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    enabled: bool,
}

impl Button {
    fn new(text: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            enabled: true,
        }
    }
    
    fn disabled(text: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            enabled: false,
        }
    }
    
    fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width &&
        py >= self.y && py <= self.y + self.height
    }
    
    fn is_hovered(&self) -> bool {
        if !self.enabled { return false; }
        let (mx, my) = virtual_mouse_position();
        self.contains(mx, my)
    }
    
    fn is_clicked(&self) -> bool {
        if !self.enabled { return false; }
        self.is_hovered() && is_mouse_button_pressed(MouseButton::Left)
    }
    
    fn is_touched(&self) -> bool {
        if !self.enabled { return false; }
        for touch in touches() {
            if touch.phase == TouchPhase::Started {
                let (vx, vy) = screen_to_virtual(touch.position.x, touch.position.y);
                if self.contains(vx, vy) {
                    return true;
                }
            }
        }
        false
    }
    
    fn is_activated(&self) -> bool {
        self.is_clicked() || self.is_touched()
    }
    
    fn draw(&self, selected: bool) {
        let base_color = if !self.enabled {
            Color::new(0.3, 0.3, 0.3, 0.8)
        } else if selected || self.is_hovered() {
            Color::new(0.3, 0.7, 0.3, 0.95)
        } else {
            Color::new(0.2, 0.5, 0.2, 0.9)
        };
        
        let border_color = if selected {
            GOLD
        } else if self.is_hovered() && self.enabled {
            WHITE
        } else {
            Color::new(0.1, 0.3, 0.1, 1.0)
        };
        
        let text_color = if self.enabled { WHITE } else { GRAY };
        
        // Draw button background
        draw_rectangle(self.x, self.y, self.width, self.height, base_color);
        draw_rectangle_lines(self.x, self.y, self.width, self.height, 3.0, border_color);
        
        // Draw text centered
        let font_size = (self.height * 0.5).min(28.0);
        let text_dim = measure_text(&self.text, None, font_size as u16, 1.0);
        let text_x = self.x + (self.width - text_dim.width) / 2.0;
        let text_y = self.y + (self.height + text_dim.height) / 2.0;
        draw_text(&self.text, text_x, text_y, font_size, text_color);
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            gravity_base: 600.0,
            fart_power_base: 380.0,
            player_size: 28.0,
            cloud_speed_initial: 180.0,
            cloud_speed_increment: 6.0,
            spawn_interval_initial: 2.0,
            spawn_interval_min: 0.8,
            difficulty_increase_every: 10,
            particle_count: 15,
            particle_lifetime: 0.7,
            shake_intensity: 12.0,
            shake_decay: 0.88,
            world_height: 2700.0,
            camera_lerp: 0.08,
            master_volume: 1.0,
            sfx_volume: 0.8,
            // Difficulty system
            speed_transition_level: 10,
            speed_slow_growth: 0.01,
            cloud_density_factor: 0.008,
            cloud_level_increment: 3,
            spawn_interval_decay: 0.08,
            hardcore_factor: 1.0,
        }
    }
}

// ============================================================================
// SOUND SYSTEM
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum SoundAction {
    Fart,       // Random from fart_1, fart_2, fart_3
    MegaFart,   // Big combo fart
    Splat,      // Death by ground
    Boom,       // Death by space
    ComboUp,    // Combo level increase
    AlertBeep,  // Danger zone beep
    GameOver,   // Game over
}

struct SoundEntry {
    sound: Sound,
    base_volume: f32,
    is_priority: bool,
}

/// Tracks an actively playing sound for ducking purposes
struct ActiveSound {
    sound: Sound,
    current_volume: f32,
    start_time: f64,
}

const DUCK_FACTOR: f32 = 0.8;      // Reduce volume by 20% per new sound
const DUCK_MIN_VOLUME: f32 = 0.1;  // Floor: never go below 10%
const SOUND_LIFETIME: f64 = 3.0;   // Cleanup sounds after 3 seconds

struct SoundRegistry {
    sounds: HashMap<SoundAction, Vec<SoundEntry>>,  // Vec for random variants
    active_sounds: Vec<ActiveSound>,  // Currently playing sounds for ducking
    master_volume: f32,
    sfx_volume: f32,
    is_muted: bool,
}

impl SoundRegistry {
    fn new(master_volume: f32, sfx_volume: f32) -> Self {
        Self {
            sounds: HashMap::new(),
            active_sounds: Vec::new(),
            master_volume,
            sfx_volume,
            is_muted: false,
        }
    }
    
    async fn load_sounds(&mut self) {
        // Sound files are optional - only load if they exist
        // We use a simple probe: try to load and silently ignore failures
        // This avoids 404 errors in browser console when sound files don't exist yet
        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, we can't check file existence, so we just try loading
            // The try_load already handles failures gracefully
            self.try_load(SoundAction::Fart, "assets/sounds/fart_1.ogg", 0.8, false).await;
            self.try_load(SoundAction::Fart, "assets/sounds/fart_2.ogg", 0.8, false).await;
            self.try_load(SoundAction::Fart, "assets/sounds/fart_3.ogg", 0.8, false).await;
            self.try_load(SoundAction::MegaFart, "assets/sounds/mega_fart.ogg", 1.0, true).await;
            self.try_load(SoundAction::Splat, "assets/sounds/splat.ogg", 1.0, true).await;
            self.try_load(SoundAction::Boom, "assets/sounds/boom.ogg", 1.0, true).await;
            self.try_load(SoundAction::ComboUp, "assets/sounds/combo_up.ogg", 0.6, false).await;
            self.try_load(SoundAction::AlertBeep, "assets/sounds/alert_beep.ogg", 0.4, false).await;
            self.try_load(SoundAction::GameOver, "assets/sounds/game_over.ogg", 1.0, true).await;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // On native, check if files exist before loading
            use std::path::Path;
            let sounds_to_load = [
                (SoundAction::Fart, "assets/sounds/fart_1.ogg", 0.8, false),
                (SoundAction::Fart, "assets/sounds/fart_2.ogg", 0.8, false),
                (SoundAction::Fart, "assets/sounds/fart_3.ogg", 0.8, false),
                (SoundAction::MegaFart, "assets/sounds/mega_fart.ogg", 1.0, true),
                (SoundAction::Splat, "assets/sounds/splat.ogg", 1.0, true),
                (SoundAction::Boom, "assets/sounds/boom.ogg", 1.0, true),
                (SoundAction::ComboUp, "assets/sounds/combo_up.ogg", 0.6, false),
                (SoundAction::AlertBeep, "assets/sounds/alert_beep.ogg", 0.4, false),
                (SoundAction::GameOver, "assets/sounds/game_over.ogg", 1.0, true),
            ];
            
            for (action, path, volume, priority) in sounds_to_load {
                if Path::new(path).exists() {
                    self.try_load(action, path, volume, priority).await;
                }
            }
        }
    }
    
    async fn try_load(&mut self, action: SoundAction, path: &str, volume: f32, priority: bool) {
        if let Ok(sound) = load_sound(path).await {
            let entry = SoundEntry {
                sound,
                base_volume: volume,
                is_priority: priority,
            };
            self.sounds.entry(action).or_insert_with(Vec::new).push(entry);
        }
    }
    
    fn play(&mut self, action: SoundAction) {
        if self.is_muted {
            return;
        }
        
        if let Some(entries) = self.sounds.get(&action) {
            if entries.is_empty() {
                return;
            }
            
            // Pick random variant if multiple
            let idx = rand::gen_range(0, entries.len());
            let entry = &entries[idx];
            
            // DUCKING: Reduce volume of ALL currently playing sounds by DUCK_FACTOR
            for active in &mut self.active_sounds {
                active.current_volume = (active.current_volume * DUCK_FACTOR).max(DUCK_MIN_VOLUME);
                let applied_volume = active.current_volume * self.master_volume;
                set_sound_volume(&active.sound, applied_volume);
            }
            
            // Calculate volume for new sound (also ducked if other sounds are playing)
            let base_volume = entry.base_volume * self.sfx_volume;
            let ducked_volume = if self.active_sounds.is_empty() {
                base_volume
            } else {
                (base_volume * DUCK_FACTOR).max(DUCK_MIN_VOLUME)
            };
            let final_volume = ducked_volume * self.master_volume;
            
            // Clone sound for tracking (Sound is Copy in macroquad)
            let sound_copy = entry.sound.clone();
            
            play_sound(&entry.sound, PlaySoundParams {
                looped: false,
                volume: final_volume,
            });
            
            // Track this sound for future ducking
            self.active_sounds.push(ActiveSound {
                sound: sound_copy,
                current_volume: ducked_volume,
                start_time: macroquad::time::get_time(),
            });
        }
    }
    
    fn update(&mut self, _dt: f32) {
        // Cleanup: Remove sounds older than SOUND_LIFETIME
        let now = macroquad::time::get_time();
        self.active_sounds.retain(|active| {
            now - active.start_time < SOUND_LIFETIME
        });
    }
    
    fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }
    
    fn set_master_volume(&mut self, vol: f32) {
        self.master_volume = vol.clamp(0.0, 1.0);
        // Update all active sounds with new master volume
        for active in &self.active_sounds {
            let applied_volume = active.current_volume * self.master_volume;
            set_sound_volume(&active.sound, applied_volume);
        }
    }
    
    fn get_master_volume(&self) -> f32 {
        self.master_volume
    }
}

// ============================================================================
// SPRITE SYSTEM
// ============================================================================

struct SpriteRegistry {
    textures: HashMap<String, Texture2D>,
}

impl SpriteRegistry {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }
    
    async fn load_sprites(&mut self) {
        let sprites = [
            "cloud", "dash", "warning", "arrow_up", "arrow_down",
            "skull", "star", "fire", "trophy", "explosion", "mountain", "rocket"
        ];
        
        for name in sprites {
            let path = format!("assets/sprites/{}.png", name);
            if let Ok(tex) = load_texture(&path).await {
                tex.set_filter(FilterMode::Linear);
                self.textures.insert(name.to_string(), tex);
            }
        }
    }
    
    fn get(&self, name: &str) -> Option<&Texture2D> {
        self.textures.get(name)
    }
    
    /// Draw sprite if available, otherwise draw fallback text
    fn draw_or_text(&self, name: &str, x: f32, y: f32, size: f32, fallback: &str, color: Color) {
        if let Some(tex) = self.get(name) {
            draw_texture_ex(
                tex,
                x - size / 2.0,
                y - size / 2.0,
                color,
                DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    ..Default::default()
                },
            );
        } else {
            draw_text(fallback, x - size / 2.0, y + size / 4.0, size * 0.8, color);
        }
    }
}

// ============================================================================
// ALTITUDE ZONES
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum AltitudeZone {
    Space,      // 0 - 25% : Low gravity, pet down = boost
    HighSky,    // 25 - 50% : Slightly low gravity
    Sky,        // 50 - 75% : Normal
    Ground,     // 75 - 100% : High gravity, pet up = boost
}

impl AltitudeZone {
    fn from_y(y: f32, world_height: f32) -> Self {
        let ratio = y / world_height;
        if ratio < 0.25 {
            AltitudeZone::Space
        } else if ratio < 0.5 {
            AltitudeZone::HighSky
        } else if ratio < 0.75 {
            AltitudeZone::Sky
        } else {
            AltitudeZone::Ground
        }
    }

    /// Visual intensity for zone-based effects (colors, particles, etc.)
    /// NOT used for gravity anymore - see calculate_gravity_multiplier()
    fn zone_intensity(&self) -> f32 {
        match self {
            AltitudeZone::Space => 0.3,
            AltitudeZone::HighSky => 0.6,
            AltitudeZone::Sky => 1.0,
            AltitudeZone::Ground => 1.4,
        }
    }

    fn fart_boost(&self, going_up: bool) -> f32 {
        // INVERTED: Space boosts UP, Ground boosts DOWN
        match self {
            AltitudeZone::Space => if going_up { 1.5 } else { 0.7 },
            AltitudeZone::HighSky => if going_up { 1.2 } else { 0.9 },
            AltitudeZone::Sky => 1.0,
            AltitudeZone::Ground => if going_up { 0.7 } else { 1.5 },
        }
    }

    fn background_color(&self) -> (Color, Color) {
        match self {
            AltitudeZone::Space => (
                Color::new(0.05, 0.05, 0.15, 1.0),
                Color::new(0.1, 0.1, 0.3, 1.0),
            ),
            AltitudeZone::HighSky => (
                Color::new(0.1, 0.2, 0.5, 1.0),
                Color::new(0.3, 0.5, 0.8, 1.0),
            ),
            AltitudeZone::Sky => (
                Color::new(0.3, 0.6, 0.9, 1.0),
                Color::new(0.5, 0.8, 1.0, 1.0),
            ),
            AltitudeZone::Ground => (
                Color::new(0.6, 0.7, 0.5, 1.0),
                Color::new(0.8, 0.6, 0.4, 1.0),
            ),
        }
    }

    fn particle_color(&self) -> Color {
        match self {
            AltitudeZone::Space => Color::new(0.3, 0.5, 1.0, 0.8),
            AltitudeZone::HighSky => Color::new(0.3, 0.8, 0.6, 0.8),
            AltitudeZone::Sky => Color::new(0.3, 0.9, 0.3, 0.8),
            AltitudeZone::Ground => Color::new(0.6, 0.5, 0.2, 0.8),
        }
    }

    fn zone_name(&self) -> &'static str {
        match self {
            AltitudeZone::Space => "* ESPACE *",
            AltitudeZone::HighSky => "~ HAUTE ALTITUDE ~",
            AltitudeZone::Sky => "= CIEL =",
            AltitudeZone::Ground => "^ SOL ^",
        }
    }
}

/// Calculate gravity multiplier based on Y position (linear interpolation)
/// y=0 (top/space): 75% gravity - floats a bit
/// y=50% (center): 100% gravity - normal
/// y=100% (bottom/ground): 125% gravity - pulls down slightly harder
fn calculate_gravity_multiplier(y: f32, world_height: f32) -> f32 {
    let ratio = (y / world_height).clamp(0.0, 1.0);
    // Linear: 0.75 at top, 1.0 at center, 1.25 at bottom
    0.75 + ratio * 0.5
}

/// Calculate combo power multiplier with diminishing returns
/// Level 2: +5%, Level 3: +4%, Level 4: +3%, Level 5: +2%, Level 6+: +1% each
fn calculate_combo_multiplier(combo_level: usize) -> f32 {
    match combo_level {
        0 | 1 | 2 => 1.0,      // No combo (need 3+ farts)
        3 => 1.05,             // +5%
        4 => 1.09,             // +5% + 4% = +9%
        5 => 1.12,             // +9% + 3% = +12%
        6 => 1.14,             // +12% + 2% = +14%
        n => 1.14 + (n - 6) as f32 * 0.01,  // +1% each after level 6
    }
}

// ============================================================================
// DEATH TYPES
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum DeathType {
    None,
    Splat,   // Hit ground - squish animation
    Explode, // Hit space - inflate and pop
    Cloud,   // Hit obstacle cloud
}

// ============================================================================
// GAME STATES
// ============================================================================

#[derive(PartialEq, Clone)]
enum GameState {
    Menu,           // Title/splash screen
    MainMenu,       // Main menu with buttons
    EnterName,      // Username input
    Leaderboard,    // Full leaderboard view
    CustomGame,     // Custom game settings (placeholder)
    Playing,
    Dying(f32),     // Animation timer
    GameOver,
}

// ============================================================================
// FLOATING TEXT (for effects like "PFFFT!", "+10", etc.)
// ============================================================================

struct FloatingText {
    text: String,
    x: f32,
    y: f32,
    vy: f32,
    lifetime: f32,
    max_lifetime: f32,
    color: Color,
    size: f32,
}

impl FloatingText {
    fn new(text: &str, x: f32, y: f32, color: Color) -> Self {
        Self {
            text: text.to_string(),
            x,
            y,
            vy: -80.0,
            lifetime: 1.2,
            max_lifetime: 1.2,
            color,
            size: rand::gen_range(24.0, 36.0),
        }
    }

    fn update(&mut self, dt: f32) {
        self.y += self.vy * dt;
        self.lifetime -= dt;
    }

    fn draw(&self, camera_y: f32) {
        let alpha = (self.lifetime / self.max_lifetime).max(0.0);
        let screen_y = self.y - camera_y + VIRTUAL_HEIGHT / 2.0;
        let color = Color::new(self.color.r, self.color.g, self.color.b, alpha);
        draw_text(&self.text, self.x, screen_y, self.size, color);
    }

    fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }
}

// ============================================================================
// ENTITIES
// ============================================================================

struct Player {
    x: f32,
    y: f32,
    velocity_y: f32,
}

impl Player {
    fn new(world_height: f32) -> Self {
        Self {
            x: 150.0,
            y: world_height / 2.0,
            velocity_y: 0.0,
        }
    }

    fn draw(&self, size: f32, screen_y: f32, zone: AltitudeZone, velocity: f32) {
        let draw_y = screen_y;
        
        // Color based on zone
        let (base_color, highlight) = match zone {
            AltitudeZone::Space => (
                Color::new(0.5, 0.6, 1.0, 1.0),
                Color::new(0.7, 0.8, 1.0, 1.0),
            ),
            AltitudeZone::HighSky => (
                Color::new(0.4, 0.8, 0.6, 1.0),
                Color::new(0.5, 0.9, 0.7, 1.0),
            ),
            AltitudeZone::Sky => (
                Color::new(0.4, 0.9, 0.4, 1.0),
                Color::new(0.5, 1.0, 0.5, 1.0),
            ),
            AltitudeZone::Ground => (
                Color::new(0.7, 0.6, 0.3, 1.0),
                Color::new(0.8, 0.7, 0.4, 1.0),
            ),
        };
        
        // Cloud body (multiple circles)
        draw_circle(self.x, draw_y, size * 0.9, base_color);
        draw_circle(self.x - size * 0.7, draw_y + size * 0.2, size * 0.6, base_color);
        draw_circle(self.x + size * 0.7, draw_y + size * 0.2, size * 0.65, base_color);
        draw_circle(self.x - size * 0.3, draw_y - size * 0.4, size * 0.5, highlight);
        draw_circle(self.x + size * 0.4, draw_y - size * 0.35, size * 0.55, highlight);
        
        // Glow effect in space
        if zone == AltitudeZone::Space {
            draw_circle(self.x, draw_y, size * 1.3, Color::new(0.5, 0.5, 1.0, 0.2));
        }
        
        // Eyes - expression based on velocity
        let eye_y = draw_y - size * 0.1;
        let eye_scale = if velocity.abs() > 200.0 { 1.3 } else { 1.0 };
        draw_circle(self.x - size * 0.25, eye_y, size * 0.15 * eye_scale, WHITE);
        draw_circle(self.x + size * 0.25, eye_y, size * 0.15 * eye_scale, WHITE);
        
        // Pupils - look in direction of movement
        let pupil_offset_y = (velocity / 1000.0).clamp(-0.05, 0.05) * size;
        draw_circle(self.x - size * 0.25, eye_y + pupil_offset_y, size * 0.08, BLACK);
        draw_circle(self.x + size * 0.25, eye_y + pupil_offset_y, size * 0.08, BLACK);
        
        // Mouth expression
        if velocity < -150.0 {
            // Farting face - O mouth
            draw_circle(self.x, draw_y + size * 0.25, size * 0.15, Color::new(0.3, 0.2, 0.2, 1.0));
        } else if velocity > 200.0 {
            // Scared face - falling fast
            draw_circle(self.x, draw_y + size * 0.28, size * 0.12, Color::new(0.2, 0.1, 0.1, 1.0));
        } else {
            // Happy smile
            draw_circle(self.x, draw_y + size * 0.32, size * 0.08, Color::new(0.3, 0.6, 0.3, 1.0));
        }
    }
}

struct ObstacleCloud {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    passed: bool,
    zone: AltitudeZone,
}

impl ObstacleCloud {
    fn new(x: f32, y: f32, width: f32, height: f32, world_height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            passed: false,
            zone: AltitudeZone::from_y(y, world_height),
        }
    }

    fn draw(&self, camera_y: f32) {
        let screen_y = self.y - camera_y + VIRTUAL_HEIGHT / 2.0;
        
        // Skip if off screen
        if screen_y < -100.0 || screen_y > VIRTUAL_HEIGHT + 100.0 {
            return;
        }
        
        // Color based on zone
        let (color, shadow) = match self.zone {
            AltitudeZone::Space => (
                Color::new(0.3, 0.3, 0.5, 0.9),
                Color::new(0.2, 0.2, 0.4, 0.8),
            ),
            AltitudeZone::HighSky => (
                Color::new(0.9, 0.9, 1.0, 0.9),
                Color::new(0.7, 0.7, 0.85, 0.8),
            ),
            AltitudeZone::Sky => (
                Color::new(1.0, 1.0, 1.0, 0.9),
                Color::new(0.85, 0.85, 0.9, 0.8),
            ),
            AltitudeZone::Ground => (
                Color::new(0.6, 0.5, 0.4, 0.9),
                Color::new(0.5, 0.4, 0.3, 0.8),
            ),
        };
        
        let cx = self.x + self.width / 2.0;
        let cy = screen_y + self.height / 2.0;
        let r = self.width.min(self.height) / 2.0;
        
        // Shadow
        draw_circle(cx + 3.0, cy + 3.0, r * 0.9, shadow);
        draw_circle(cx - r * 0.5 + 3.0, cy + 3.0, r * 0.6, shadow);
        draw_circle(cx + r * 0.5 + 3.0, cy + 3.0, r * 0.55, shadow);
        
        // Body
        draw_circle(cx, cy, r * 0.9, color);
        draw_circle(cx - r * 0.6, cy + r * 0.1, r * 0.6, color);
        draw_circle(cx + r * 0.6, cy + r * 0.1, r * 0.55, color);
        draw_circle(cx - r * 0.2, cy - r * 0.4, r * 0.45, color);
        draw_circle(cx + r * 0.3, cy - r * 0.35, r * 0.5, color);
    }

    fn collides_with(&self, player: &Player, player_size: f32) -> bool {
        let closest_x = player.x.clamp(self.x, self.x + self.width);
        let closest_y = player.y.clamp(self.y, self.y + self.height);
        let distance = ((player.x - closest_x).powi(2) + (player.y - closest_y).powi(2)).sqrt();
        distance < player_size * 0.7
    }
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    lifetime: f32,
    max_lifetime: f32,
    size: f32,
    color: Color,
}

impl Particle {
    fn new(x: f32, y: f32, lifetime: f32, direction_up: bool, color: Color) -> Self {
        // Particles go opposite to movement direction
        let base_angle: f32 = if direction_up {
            rand::gen_range(1.2, 1.9) // Down-ish (player going up)
        } else {
            rand::gen_range(-1.9, -1.2) // Up-ish (player going down)
        };
        let speed: f32 = rand::gen_range(100.0, 250.0);
        Self {
            x,
            y,
            vx: base_angle.cos() * speed * 0.3,
            vy: base_angle.sin() * speed,
            lifetime,
            max_lifetime: lifetime,
            size: rand::gen_range(5.0, 12.0),
            color,
        }
    }

    fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.lifetime -= dt;
    }

    fn draw(&self, camera_y: f32) {
        let alpha = (self.lifetime / self.max_lifetime).max(0.0);
        let screen_y = self.y - camera_y + VIRTUAL_HEIGHT / 2.0;
        let color = Color::new(self.color.r, self.color.g, self.color.b, alpha * self.color.a);
        draw_circle(self.x, screen_y, self.size * alpha, color);
    }

    fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }
}

// ============================================================================
// LEADERBOARD
// ============================================================================

#[derive(Serialize, Deserialize, Clone)]
struct LeaderboardEntry {
    name: String,
    score: u32,
}

/// Leaderboard state for async fetching
static mut LEADERBOARD_CACHE: Option<Vec<LeaderboardEntry>> = None;
static mut LEADERBOARD_FETCHING: bool = false;

/// Check if leaderboard is currently loading
fn is_leaderboard_loading() -> bool {
    unsafe { LEADERBOARD_FETCHING }
}

/// Start fetching leaderboard from platform API (async)
fn start_leaderboard_fetch() {
    #[cfg(target_arch = "wasm32")]
    {
        unsafe {
            if !LEADERBOARD_FETCHING {
                LEADERBOARD_FETCHING = true;
                js_platform::fetch_leaderboard();
            }
        }
    }
}

/// Check if leaderboard is ready and get it
fn poll_leaderboard() -> Option<Vec<LeaderboardEntry>> {
    #[cfg(target_arch = "wasm32")]
    {
        if js_platform::is_leaderboard_ready() {
            unsafe {
                LEADERBOARD_FETCHING = false;
            }
            let count = js_platform::get_leaderboard_count();
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let (name, score) = js_platform::get_leaderboard_entry(i);
                entries.push(LeaderboardEntry { name, score });
            }
            js_platform::reset_leaderboard();
            return Some(entries);
        }
        None
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop: return mock data
        Some(vec![
            LeaderboardEntry { name: "SpaceFarter".to_string(), score: 1500 },
            LeaderboardEntry { name: "CloudKing".to_string(), score: 999 },
            LeaderboardEntry { name: "PetMaster".to_string(), score: 500 },
        ])
    }
}

/// Get cached leaderboard or empty vec
fn get_leaderboard() -> Vec<LeaderboardEntry> {
    unsafe {
        LEADERBOARD_CACHE.clone().unwrap_or_default()
    }
}

/// Update leaderboard cache
fn update_leaderboard_cache(entries: Vec<LeaderboardEntry>) {
    unsafe {
        LEADERBOARD_CACHE = Some(entries);
    }
}

/// Submit score to platform API
fn submit_score(name: &str, score: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::submit_score(name, score);
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("[Leaderboard] Score submitted: {} - {}", name, score);
        let _ = (name, score);
    }
}

/// Get persisted high score from sessionStorage or platform
fn get_stored_high_score() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::get_high_score()
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// Save high score to sessionStorage
fn save_high_score(score: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::save_high_score(score);
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = score;
    }
}

/// Submit detailed score to platform API (with game stats)
fn submit_score_full(
    name: &str, score: u32,
    diff_level: u32, combo_max: u32, fart_count: u32,
    duration_secs: f32, death_type: &str, zone: &str,
) {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::submit_score_full(
            name, score, diff_level, combo_max, fart_count,
            duration_secs, death_type, zone,
        );
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("[Platform] Score submitted: {} - {} (lvl:{}, combo:{}, farts:{}, {:.1}s, {}, {})",
            name, score, diff_level, combo_max, fart_count, duration_secs, death_type, zone);
        let _ = (name, score, diff_level, combo_max, fart_count, duration_secs, death_type, zone);
    }
}

/// Check if platform is configured and user is authenticated
fn is_platform_connected() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::is_platform_configured() && js_platform::is_authenticated()
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Get the platform username (or "Anonyme")
fn get_platform_username() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_platform::get_auth_username()
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        "Anonyme".to_string()
    }
}

/// Get platform config override JSON (partial)
fn get_platform_config_override() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if js_platform::is_platform_config_ready() {
            js_platform::get_platform_config_json()
        } else {
            None
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

// ============================================================================
// CLICK EFFECT (Visual feedback for mouse/touch)
// ============================================================================

struct ClickEffect {
    x: f32,
    y: f32,
    timer: f32,
    max_time: f32,
}

impl ClickEffect {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y, timer: 0.4, max_time: 0.4 }
    }
    
    fn update(&mut self, dt: f32) {
        self.timer -= dt;
    }
    
    fn is_alive(&self) -> bool {
        self.timer > 0.0
    }
    
    fn draw(&self) {
        let progress = 1.0 - (self.timer / self.max_time);
        let radius = 8.0 + progress * 35.0;  // Grows from 8 to 43
        let alpha = (1.0 - progress) * 0.8;   // Fades out
        let thickness = 3.0 * (1.0 - progress * 0.5);  // Thins slightly
        
        // Outer ring - golden yellow
        draw_circle_lines(self.x, self.y, radius, thickness, 
            Color::new(1.0, 0.85, 0.2, alpha));
        
        // Inner dot that fades quickly
        if progress < 0.3 {
            let dot_alpha = (0.3 - progress) / 0.3 * 0.6;
            draw_circle(self.x, self.y, 4.0, Color::new(1.0, 1.0, 0.5, dot_alpha));
        }
    }
}

// ============================================================================
// MAIN GAME
// ============================================================================

struct Game {
    config: GameConfig,
    state: GameState,
    player: Player,
    obstacles: Vec<ObstacleCloud>,
    particles: Vec<Particle>,
    floating_texts: Vec<FloatingText>,
    score: f32,              // Now float for time-based
    high_score: u32,
    player_name: String,
    name_input: String,
    shake_offset: Vec2,
    spawn_timer: f32,
    current_speed: f32,
    current_spawn_interval: f32,
    min_clouds: u32,         // Minimum cloud count for hybrid spawn
    leaderboard: Vec<LeaderboardEntry>,
    fart_count: u32,
    camera_y: f32,
    distance_traveled: f32,
    current_zone: AltitudeZone,
    // Combo system
    fart_times: Vec<f32>,    // Timestamps of recent farts
    combo_multiplier: f32,
    combo_count: usize,      // Raw combo count (3, 4, 5...)
    combo_display_timer: f32,
    // Death
    death_type: DeathType,
    death_animation_scale: f32,
    difficulty_level: u32,
    play_time: f32,
    // Alert system
    last_beep_time: f64,
    beep_flash: f32,
    // Sound events queue (to be played by main loop)
    pending_sounds: Vec<SoundAction>,
    // Menu navigation
    selected_button: usize,
    // Click visual feedback
    click_effects: Vec<ClickEffect>,
    // Platform connection
    is_connected: bool,
    // Track max combo for score submission
    combo_max: u32,
}

impl Game {
    fn new(config: GameConfig) -> Self {
        let world_height = config.world_height;
        let initial_min_clouds = calculate_min_clouds(1, &config);
        let stored_high = get_stored_high_score();
        let connected = is_platform_connected();
        // If connected, use platform username
        let platform_name = if connected {
            get_platform_username()
        } else {
            String::new()
        };
        Self {
            player: Player::new(world_height),
            obstacles: Vec::new(),
            particles: Vec::new(),
            floating_texts: Vec::new(),
            score: 0.0,
            high_score: stored_high,
            player_name: platform_name,
            name_input: String::new(),
            shake_offset: Vec2::ZERO,
            spawn_timer: 0.0,
            current_speed: config.cloud_speed_initial,
            current_spawn_interval: config.spawn_interval_initial,
            min_clouds: initial_min_clouds,
            leaderboard: get_leaderboard(),
            fart_count: 0,
            camera_y: world_height / 2.0,
            distance_traveled: 0.0,
            current_zone: AltitudeZone::Sky,
            state: GameState::Menu,
            // Combo system
            fart_times: Vec::new(),
            combo_multiplier: 1.0,
            combo_count: 0,
            combo_display_timer: 0.0,
            // Death
            death_type: DeathType::None,
            death_animation_scale: 1.0,
            difficulty_level: 1,
            play_time: 0.0,
            // Alert system
            last_beep_time: 0.0,
            beep_flash: 0.0,
            // Sound events
            pending_sounds: Vec::new(),
            // Menu navigation
            selected_button: 0,
            // Click effects
            click_effects: Vec::new(),
            // Platform
            is_connected: connected,
            combo_max: 0,
            config,
        }
    }

    fn reset(&mut self) {
        self.player = Player::new(self.config.world_height);
        self.obstacles.clear();
        self.particles.clear();
        self.floating_texts.clear();
        self.score = 0.0;
        self.fart_count = 0;
        self.spawn_timer = 0.0;
        self.current_speed = self.config.cloud_speed_initial;
        self.current_spawn_interval = self.config.spawn_interval_initial;
        self.min_clouds = calculate_min_clouds(1, &self.config);
        self.shake_offset = Vec2::ZERO;
        self.camera_y = self.config.world_height / 2.0;
        self.distance_traveled = 0.0;
        self.current_zone = AltitudeZone::Sky;
        // Combo system
        self.fart_times.clear();
        self.combo_multiplier = 1.0;
        self.combo_count = 0;
        self.combo_display_timer = 0.0;
        // Death
        self.death_type = DeathType::None;
        self.death_animation_scale = 1.0;
        self.difficulty_level = 1;
        self.play_time = 0.0;
        // Platform
        self.combo_max = 0;
        self.is_connected = is_platform_connected();
    }

    fn fart(&mut self, direction_up: bool) {
        let zone = AltitudeZone::from_y(self.player.y, self.config.world_height);
        let boost = zone.fart_boost(direction_up);
        let power = self.config.fart_power_base * boost * self.combo_multiplier;
        
        // Apply velocity
        if direction_up {
            self.player.velocity_y = -power;
        } else {
            self.player.velocity_y = power;
        }
        
        self.fart_count += 1;
        
        // Track fart timing for combo
        self.fart_times.push(self.play_time);
        // Keep only farts from last 3 seconds
        self.fart_times.retain(|&t| self.play_time - t < 3.0);
        
        // Check for combo (3+ farts in 3 seconds)
        let is_mega = self.fart_times.len() >= 5;
        if self.fart_times.len() >= 3 {
            // Degressive combo - diminishing returns on power boost
            let combo_level = self.fart_times.len();
            let old_combo = self.combo_count;
            self.combo_count = combo_level;
            self.combo_multiplier = calculate_combo_multiplier(combo_level);
            self.combo_display_timer = 2.0;
            // Track max combo for score submission
            if combo_level as u32 > self.combo_max {
                self.combo_max = combo_level as u32;
            }
            
            // Play combo up sound if new level
            if combo_level > old_combo {
                self.pending_sounds.push(SoundAction::ComboUp);
            }
            
            // Bonus points for combo (still scales linearly for score)
            let bonus = combo_level as f32 * 10.0 * self.difficulty_level as f32;
            self.score += bonus;
            
            // Show combo text: "5x COMBO! +50"
            let pet_boost = ((self.combo_multiplier - 1.0) * 100.0) as i32;
            let combo_text = format!("{}x COMBO! +{:.0} (+{}% pet)", combo_level, bonus, pet_boost);
            self.floating_texts.push(FloatingText::new(
                &combo_text,
                self.player.x,
                self.player.y - 50.0,
                Color::new(1.0, 0.5, 0.0, 1.0),
            ));
        }
        
        // Queue sound - mega fart for big combo, otherwise random fart
        if is_mega {
            self.pending_sounds.push(SoundAction::MegaFart);
        } else {
            self.pending_sounds.push(SoundAction::Fart);
        }
        
        // Spawn particles
        let particle_color = zone.particle_color();
        for _ in 0..self.config.particle_count {
            let offset_x = rand::gen_range(-15.0, 15.0);
            let offset_y = if direction_up { 
                self.config.player_size * 0.8 
            } else { 
                -self.config.player_size * 0.8 
            };
            self.particles.push(Particle::new(
                self.player.x + offset_x,
                self.player.y + offset_y,
                self.config.particle_lifetime,
                direction_up,
                particle_color,
            ));
        }
        
        // Floating text with random variety
        let fart_texts_normal = ["PFFFT!", "PROUT!", "BRAAAP!", "FLOOOP!", "~pff~"];
        let fart_texts_mega = ["MEGA PET!", "SPLORTCH!", "BRAAAAP!!", "KABOOM PET!"];
        let fart_text = if boost > 1.2 {
            fart_texts_mega[rand::gen_range(0, fart_texts_mega.len())]
        } else {
            fart_texts_normal[rand::gen_range(0, fart_texts_normal.len())]
        };
        self.floating_texts.push(FloatingText::new(
            fart_text,
            self.player.x + rand::gen_range(-30.0, 30.0),
            self.player.y,
            if boost > 1.2 { GOLD } else { Color::new(0.5, 1.0, 0.5, 1.0) },
        ));
        
        // Screen shake (more intense with boost)
        let shake_mult = if boost > 1.2 { 1.5 } else { 1.0 };
        self.shake_offset = Vec2::new(
            rand::gen_range(-1.0, 1.0) * self.config.shake_intensity * shake_mult,
            rand::gen_range(-1.0, 1.0) * self.config.shake_intensity * shake_mult,
        );
    }

    fn spawn_obstacle(&mut self) {
        self.spawn_obstacle_with_offset(0.0);
    }
    
    fn spawn_obstacle_with_offset(&mut self, x_offset: f32) {
        // Spawn at random Y within world bounds with some margin
        let margin = 150.0;
        let y = rand::gen_range(margin, self.config.world_height - margin);
        let cloud_size = rand::gen_range(45.0, 75.0);
        
        self.obstacles.push(ObstacleCloud::new(
            VIRTUAL_WIDTH + 50.0 + x_offset,
            y,
            cloud_size,
            cloud_size * 0.7,
            self.config.world_height,
        ));
    }

    fn update_difficulty(&mut self) {
        // Level increases based on config (default every 10 seconds)
        let time_per_level = self.config.difficulty_increase_every as f32;
        self.difficulty_level = 1 + (self.play_time / time_per_level) as u32;
        
        // Degressive speed: fast early, slow growth after transition
        self.current_speed = calculate_speed(self.difficulty_level, &self.config);
        
        // Spawn interval: decreases with level, respects hardcore-adjusted minimum
        let adjusted_min = calculate_spawn_interval_min(&self.config);
        self.current_spawn_interval = (self.config.spawn_interval_initial 
            - self.difficulty_level as f32 * self.config.spawn_interval_decay)
            .max(adjusted_min);
        
        // Minimum cloud count: increases with level
        self.min_clouds = calculate_min_clouds(self.difficulty_level, &self.config);
    }

    fn die(&mut self, death_type: DeathType) {
        self.death_type = death_type;
        self.death_animation_scale = 1.0;
        self.state = GameState::Dying(0.0);
        
        // Queue death sound
        match death_type {
            DeathType::Splat => self.pending_sounds.push(SoundAction::Splat),
            DeathType::Explode => self.pending_sounds.push(SoundAction::Boom),
            _ => {}
        }
        
        // Add death particles
        let color = match death_type {
            DeathType::Splat => Color::new(0.5, 0.3, 0.1, 1.0), // Brown splat
            DeathType::Explode => Color::new(1.0, 0.8, 0.3, 1.0), // Golden explosion
            _ => Color::new(0.5, 0.5, 0.5, 1.0),
        };
        
        for _ in 0..30 {
            let angle: f32 = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
            let speed: f32 = rand::gen_range(100.0, 300.0);
            self.particles.push(Particle {
                x: self.player.x,
                y: self.player.y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                lifetime: 1.5,
                max_lifetime: 1.5,
                size: rand::gen_range(8.0, 20.0),
                color,
            });
        }
    }

    fn update(&mut self, dt: f32, sounds: &mut SoundRegistry) {
        // Check for any touch as a click
        let touch_started = touches().iter().any(|t| t.phase == TouchPhase::Started);
        let any_click = is_mouse_button_pressed(MouseButton::Left) || touch_started;
        
        match self.state.clone() {
            GameState::Menu => {
                // Splash screen - any input goes to main menu
                if is_key_pressed(KeyCode::Space) || any_click {
                    self.selected_button = 0;
                    // Start fetching leaderboard in background
                    start_leaderboard_fetch();
                    self.state = GameState::MainMenu;
                }
            }
            GameState::MainMenu => {
                // Button dimensions (must match draw_main_menu)
                let cx = VIRTUAL_WIDTH / 2.0;
                let btn_width = scaled(280.0);
                let btn_height = scaled(50.0);
                let btn_x = cx - btn_width / 2.0;
                let btn_start_y = scaled(160.0);
                let btn_spacing = scaled(65.0);
                
                // Create button hitboxes
                let btn_play = Button::new("", btn_x, btn_start_y, btn_width, btn_height);
                let btn_leaderboard = Button::new("", btn_x, btn_start_y + btn_spacing, btn_width, btn_height);
                let btn_custom = Button::disabled("", btn_x, btn_start_y + btn_spacing * 2.0, btn_width, btn_height);
                
                // Keyboard navigation
                let button_count = 3; // JOUER, LEADERBOARD, PARTIE PERSO
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    self.selected_button = (self.selected_button + button_count - 1) % button_count;
                }
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    self.selected_button = (self.selected_button + 1) % button_count;
                }
                
                // Update selected button on hover
                if btn_play.is_hovered() { self.selected_button = 0; }
                if btn_leaderboard.is_hovered() { self.selected_button = 1; }
                // btn_custom is disabled, don't update on hover
                
                // Check for activation (keyboard or click)
                let play_activated = btn_play.is_activated() || (self.selected_button == 0 && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)));
                let leaderboard_activated = btn_leaderboard.is_activated() || (self.selected_button == 1 && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)));
                let custom_activated = btn_custom.is_activated() || (self.selected_button == 2 && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)));
                
                if play_activated {
                    if self.player_name.is_empty() && !self.is_connected {
                        self.state = GameState::EnterName;
                        // Trigger mobile keyboard on WASM
                        #[cfg(target_arch = "wasm32")]
                        js_keyboard::show_keyboard();
                    } else {
                        // If connected but no name yet, use platform username
                        if self.player_name.is_empty() && self.is_connected {
                            self.player_name = get_platform_username();
                        }
                        self.reset();
                        self.state = GameState::Playing;
                    }
                } else if leaderboard_activated {
                    // Start fetching if not already (only if connected)
                    if self.is_connected {
                        start_leaderboard_fetch();
                    }
                    self.state = GameState::Leaderboard;
                } else if custom_activated {
                    self.state = GameState::CustomGame;
                }
                
                // Poll for leaderboard data in background
                if let Some(entries) = poll_leaderboard() {
                    update_leaderboard_cache(entries.clone());
                    self.leaderboard = entries;
                }
                
                // Escape goes back to splash
                if is_key_pressed(KeyCode::Escape) {
                    self.state = GameState::Menu;
                }
            }
            GameState::Leaderboard => {
                // Poll for leaderboard data
                if let Some(entries) = poll_leaderboard() {
                    update_leaderboard_cache(entries.clone());
                    self.leaderboard = entries;
                }
                
                // Any input returns to main menu
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Space) || any_click {
                    self.state = GameState::MainMenu;
                }
            }
            GameState::CustomGame => {
                // Placeholder - escape returns to main menu
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Space) {
                    self.state = GameState::MainMenu;
                }
            }
            GameState::EnterName => {
                // On WASM, use the mobile keyboard overlay
                #[cfg(target_arch = "wasm32")]
                {
                    // Check if mobile input was confirmed
                    if js_keyboard::is_confirmed() {
                        let input = js_keyboard::get_input();
                        js_keyboard::reset();
                        js_keyboard::hide_keyboard();
                        
                        if input.is_empty() || input == "Anonymous" {
                            self.player_name = "Anonymous".to_string();
                        } else {
                            self.player_name = input;
                        }
                        self.name_input.clear();
                        self.reset();
                        self.state = GameState::Playing;
                    }
                }
                
                // On desktop (non-WASM), use keyboard input
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(c) = get_char_pressed() {
                        if c.is_alphanumeric() && self.name_input.len() < 12 {
                            self.name_input.push(c);
                        }
                    }
                    if is_key_pressed(KeyCode::Backspace) {
                        self.name_input.pop();
                    }
                    if is_key_pressed(KeyCode::Enter) && !self.name_input.is_empty() {
                        self.player_name = self.name_input.clone();
                        self.name_input.clear();
                        self.reset();
                        self.state = GameState::Playing;
                    }
                }
                
                // Escape goes back to menu on any platform
                if is_key_pressed(KeyCode::Escape) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        js_keyboard::hide_keyboard();
                        js_keyboard::reset();
                    }
                    self.name_input.clear();
                    self.state = GameState::MainMenu;
                }
            }
            GameState::Playing => {
                // Update play time
                self.play_time += dt;
                
                // Update beep flash effect (decay quickly)
                self.beep_flash = (self.beep_flash - dt * 5.0).max(0.0);
                
                // Check for danger zone beeps
                let ratio = self.player.y / self.config.world_height;
                let in_danger = ratio < 0.25 || ratio > 0.75;
                if in_danger {
                    let urgency = if ratio < 0.25 { 1.0 - (ratio / 0.25) } else { (ratio - 0.75) / 0.25 };
                    let beep_interval = 0.5 - urgency * 0.45;
                    if (get_time() - self.last_beep_time) > beep_interval as f64 {
                        self.last_beep_time = get_time();
                        self.beep_flash = 1.0; // Trigger flash
                        self.pending_sounds.push(SoundAction::AlertBeep);
                    }
                }
                
                // Update current zone
                self.current_zone = AltitudeZone::from_y(self.player.y, self.config.world_height);
                
                // TIME-BASED SCORE: 1 point/sec × difficulty × combo
                self.score += dt * self.difficulty_level as f32 * self.combo_multiplier;
                
                // Decay combo multiplier over time
                if self.combo_display_timer > 0.0 {
                    self.combo_display_timer -= dt;
                } else if self.combo_multiplier > 1.0 {
                    self.combo_multiplier = (self.combo_multiplier - dt * 0.5).max(1.0);
                }
                
                // Input - directional fart (mouse)
                if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
                    // Determine direction based on mouse position relative to player
                    let (mouse_x, mouse_y) = virtual_mouse_position();
                    let player_screen_y = self.player.y - self.camera_y + VIRTUAL_HEIGHT / 2.0;
                    let direction_up = mouse_y < player_screen_y;
                    self.fart(direction_up);
                    // Add click effect at mouse position
                    self.click_effects.push(ClickEffect::new(mouse_x, mouse_y));
                }
                
                // Touch input for fart
                for touch in touches() {
                    if touch.phase == TouchPhase::Started {
                        let (touch_x, touch_y) = screen_to_virtual(touch.position.x, touch.position.y);
                        let player_screen_y = self.player.y - self.camera_y + VIRTUAL_HEIGHT / 2.0;
                        let direction_up = touch_y < player_screen_y;
                        self.fart(direction_up);
                        // Add click effect at touch position
                        self.click_effects.push(ClickEffect::new(touch_x, touch_y));
                        break; // Only process one touch
                    }
                }
                
                // Also allow keyboard controls
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    self.fart(true);
                }
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    self.fart(false);
                }

                // Physics with progressive gravity (linear based on Y position)
                let gravity_mult = calculate_gravity_multiplier(self.player.y, self.config.world_height);
                let gravity = self.config.gravity_base * gravity_mult;
                self.player.velocity_y += gravity * dt;
                self.player.y += self.player.velocity_y * dt;
                
                // DEATH AT BOUNDARIES instead of bounce
                let death_margin = 30.0;
                if self.player.y < death_margin {
                    // Hit space - EXPLODE (inflate and pop)
                    self.player.y = death_margin;
                    self.die(DeathType::Explode);
                    return;
                }
                if self.player.y > self.config.world_height - death_margin {
                    // Hit ground - SPLAT
                    self.player.y = self.config.world_height - death_margin;
                    self.die(DeathType::Splat);
                    return;
                }

                // Camera follows player with lerp
                let target_camera_y = self.player.y;
                self.camera_y += (target_camera_y - self.camera_y) * self.config.camera_lerp;
                
                // Clamp camera
                let half_screen = VIRTUAL_HEIGHT / 2.0;
                self.camera_y = self.camera_y.clamp(half_screen, self.config.world_height - half_screen);

                // Update difficulty based on time
                self.update_difficulty();

                // Hybrid spawn system: timer-based + minimum cloud count
                // Grace period: level 1 uses timer only (no min_clouds enforcement)
                // Rate limiter: minimum 0.25s between any spawns to prevent walls
                // Hardcore factor reduces the minimum delay for harder modes
                self.spawn_timer += dt;
                
                let grace_period = self.difficulty_level <= 1;
                let min_spawn_delay = 0.25 / self.config.hardcore_factor;
                
                let need_spawn_timer = self.spawn_timer >= self.current_spawn_interval;
                let need_spawn_density = !grace_period 
                    && (self.obstacles.len() as u32) < self.min_clouds
                    && self.spawn_timer >= min_spawn_delay;
                
                if need_spawn_timer || need_spawn_density {
                    // If it's a catch-up spawn (density), add random X offset to spread clouds
                    if need_spawn_density && !need_spawn_timer {
                        let offset = rand::gen_range(50.0, 200.0);
                        self.spawn_obstacle_with_offset(offset);
                    } else {
                        self.spawn_obstacle();
                    }
                    self.spawn_timer = 0.0;
                }

                // Update obstacles
                let current_speed = self.current_speed;
                for obs in &mut self.obstacles {
                    obs.x -= current_speed * dt;
                }

                // Distance score
                self.distance_traveled += current_speed * dt;

                // Remove off-screen obstacles
                self.obstacles.retain(|obs| obs.x + obs.width > -100.0);

                // Update particles
                for particle in &mut self.particles {
                    particle.update(dt);
                }
                self.particles.retain(|p| !p.is_dead());

                // Update floating texts
                for ft in &mut self.floating_texts {
                    ft.update(dt);
                }
                self.floating_texts.retain(|ft| !ft.is_dead());

                // Update click effects
                for effect in &mut self.click_effects {
                    effect.update(dt);
                }
                self.click_effects.retain(|e| e.is_alive());

                // Decay shake
                self.shake_offset *= self.config.shake_decay;

                // Collision detection with obstacles
                for obs in &self.obstacles {
                    if obs.collides_with(&self.player, self.config.player_size) {
                        self.die(DeathType::Cloud);
                        return;
                    }
                }
            }
            GameState::Dying(timer) => {
                let new_timer = timer + dt;
                
                // Animate death
                match self.death_type {
                    DeathType::Splat => {
                        // Squish animation - flatten horizontally
                        self.death_animation_scale = 1.0 + new_timer * 2.0; // Expand horizontally
                    }
                    DeathType::Explode => {
                        // Inflate then pop
                        if new_timer < 0.5 {
                            self.death_animation_scale = 1.0 + new_timer * 3.0; // Inflate
                        } else {
                            self.death_animation_scale = 0.0; // Pop!
                        }
                    }
                    _ => {}
                }
                
                // Update particles during death
                for particle in &mut self.particles {
                    particle.update(dt);
                }
                self.particles.retain(|p| !p.is_dead());
                
                // Transition to game over after animation
                if new_timer >= 1.0 {
                    let total_score = self.score as u32;
                    if total_score > self.high_score {
                        self.high_score = total_score;
                        save_high_score(total_score);
                    }
                    // Submit score — full details if connected, basic otherwise
                    if self.is_connected {
                        let death_str = match self.death_type {
                            DeathType::Splat => "splat",
                            DeathType::Explode => "explode",
                            DeathType::Cloud => "cloud",
                            DeathType::None => "none",
                        };
                        let zone_str = match self.current_zone {
                            AltitudeZone::Space => "space",
                            AltitudeZone::HighSky => "high_sky",
                            AltitudeZone::Sky => "sky",
                            AltitudeZone::Ground => "ground",
                        };
                        submit_score_full(
                            &self.player_name, total_score,
                            self.difficulty_level, self.combo_max,
                            self.fart_count, self.play_time,
                            death_str, zone_str,
                        );
                    }
                    // Start async leaderboard fetch
                    if self.is_connected {
                        start_leaderboard_fetch();
                    }
                    self.pending_sounds.push(SoundAction::GameOver);
                    self.state = GameState::GameOver;
                } else {
                    self.state = GameState::Dying(new_timer);
                }
            }
            GameState::GameOver => {
                // Poll for leaderboard data (async fetch)
                if let Some(entries) = poll_leaderboard() {
                    update_leaderboard_cache(entries.clone());
                    self.leaderboard = entries;
                }
                
                if is_key_pressed(KeyCode::Space) || any_click {
                    self.reset();
                    self.state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Escape) {
                    self.state = GameState::MainMenu;
                }
            }
        }
    }

    fn draw_game(&self, _sprites: &SpriteRegistry, sounds: &SoundRegistry) {
        // Draw background based on camera position (zone gradient)
        self.draw_background();
        
        // Apply shake
        let offset = self.shake_offset;

        match &self.state {
            GameState::Menu => {
                self.draw_menu();
            }
            GameState::MainMenu => {
                self.draw_main_menu();
            }
            GameState::Leaderboard => {
                self.draw_leaderboard_screen();
            }
            GameState::CustomGame => {
                self.draw_custom_game();
            }
            GameState::EnterName => {
                self.draw_name_input();
            }
            GameState::Playing => {
                self.draw_gameplay(offset, 1.0, sounds);
            }
            GameState::Dying(_) => {
                self.draw_gameplay(offset, self.death_animation_scale, sounds);
                self.draw_death_message();
            }
            GameState::GameOver => {
                self.draw_gameplay(offset, 0.0, sounds); // Don't draw player
                self.draw_game_over();
            }
        }
    }
    
    fn draw_death_message(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let cy = VIRTUAL_HEIGHT / 2.0;
        
        let message = match self.death_type {
            DeathType::Splat => "*SPLAT!*",
            DeathType::Explode => "*BOOM!*",
            DeathType::Cloud => "*POUF!*",
            DeathType::None => "",
        };
        
        let color = match self.death_type {
            DeathType::Splat => Color::new(0.6, 0.3, 0.1, 1.0),
            DeathType::Explode => Color::new(1.0, 0.8, 0.2, 1.0),
            DeathType::Cloud => Color::new(0.8, 0.8, 0.8, 1.0),
            DeathType::None => WHITE,
        };
        
        let font_size = scaled_font(60.0);
        let dim = measure_text(message, None, font_size as u16, 1.0);
        draw_text(message, cx - dim.width / 2.0, cy, font_size, color);
    }

    fn draw_background(&self) {
        // Continuous color interpolation based on camera Y position
        let ratio = (self.camera_y / self.config.world_height).clamp(0.0, 1.0);
        let zone = AltitudeZone::from_y(self.camera_y, self.config.world_height);
        
        // Define zone colors for interpolation
        let space_c1 = (0.05, 0.05, 0.15);
        let space_c2 = (0.1, 0.1, 0.3);
        let highsky_c1 = (0.1, 0.2, 0.5);
        let highsky_c2 = (0.3, 0.5, 0.8);
        let sky_c1 = (0.3, 0.6, 0.9);
        let sky_c2 = (0.5, 0.8, 1.0);
        let ground_c1 = (0.6, 0.7, 0.5);
        let ground_c2 = (0.8, 0.6, 0.4);
        
        // Interpolate between zones based on ratio
        let (c1, c2) = if ratio < 0.25 {
            let t = ratio / 0.25;
            (
                (space_c1.0 + (highsky_c1.0 - space_c1.0) * t,
                 space_c1.1 + (highsky_c1.1 - space_c1.1) * t,
                 space_c1.2 + (highsky_c1.2 - space_c1.2) * t),
                (space_c2.0 + (highsky_c2.0 - space_c2.0) * t,
                 space_c2.1 + (highsky_c2.1 - space_c2.1) * t,
                 space_c2.2 + (highsky_c2.2 - space_c2.2) * t),
            )
        } else if ratio < 0.5 {
            let t = (ratio - 0.25) / 0.25;
            (
                (highsky_c1.0 + (sky_c1.0 - highsky_c1.0) * t,
                 highsky_c1.1 + (sky_c1.1 - highsky_c1.1) * t,
                 highsky_c1.2 + (sky_c1.2 - highsky_c1.2) * t),
                (highsky_c2.0 + (sky_c2.0 - highsky_c2.0) * t,
                 highsky_c2.1 + (sky_c2.1 - highsky_c2.1) * t,
                 highsky_c2.2 + (sky_c2.2 - highsky_c2.2) * t),
            )
        } else if ratio < 0.75 {
            let t = (ratio - 0.5) / 0.25;
            (
                (sky_c1.0 + (ground_c1.0 - sky_c1.0) * t,
                 sky_c1.1 + (ground_c1.1 - sky_c1.1) * t,
                 sky_c1.2 + (ground_c1.2 - sky_c1.2) * t),
                (sky_c2.0 + (ground_c2.0 - sky_c2.0) * t,
                 sky_c2.1 + (ground_c2.1 - sky_c2.1) * t,
                 sky_c2.2 + (ground_c2.2 - sky_c2.2) * t),
            )
        } else {
            (ground_c1, ground_c2)
        };
        
        let color1 = Color::new(c1.0, c1.1, c1.2, 1.0);
        let color2 = Color::new(c2.0, c2.1, c2.2, 1.0);
        
        // Gradient effect
        for i in 0..20 {
            let t = i as f32 / 20.0;
            let y = t * VIRTUAL_HEIGHT;
            let color = Color::new(
                color1.r + (color2.r - color1.r) * t,
                color1.g + (color2.g - color1.g) * t,
                color1.b + (color2.b - color1.b) * t,
                1.0,
            );
            draw_rectangle(0.0, y, VIRTUAL_WIDTH, VIRTUAL_HEIGHT / 20.0 + 1.0, color);
        }
        
        // Stars - only visible in Space and HighSky zones (non-progressive)
        if zone == AltitudeZone::Space || zone == AltitudeZone::HighSky {
            let star_alpha = if zone == AltitudeZone::Space { 0.9 } else { 0.5 };
            for i in 0..30 {
                let x = ((i * 137) % 800) as f32;
                let y = ((i * 251) % 600) as f32;
                let size = (i % 3) as f32 + 1.0;
                let twinkle = ((get_time() * 3.0 + i as f64).sin() * 0.3 + 0.7) as f32;
                draw_circle(x, y, size, Color::new(1.0, 1.0, 1.0, star_alpha * twinkle));
            }
        }
        
        // Ground elements - positioned in world coordinates, converted to screen
        // The ground is at world_height, convert to screen Y
        let ground_world_y = self.config.world_height;
        let ground_screen_y = ground_world_y - self.camera_y + VIRTUAL_HEIGHT / 2.0;
        
        // Only draw if ground is visible on screen (screen_y < screen_height + some margin)
        if ground_screen_y < VIRTUAL_HEIGHT + 150.0 && ground_screen_y > 0.0 {
            for i in 0..5 {
                let x = (i as f32 * 200.0) - 50.0;
                let height = 80.0 + (i as f32 * 30.0) % 60.0;
                draw_triangle(
                    Vec2::new(x, ground_screen_y),
                    Vec2::new(x + 100.0, ground_screen_y - height),
                    Vec2::new(x + 200.0, ground_screen_y),
                    Color::new(0.3, 0.25, 0.2, 0.7),
                );
            }
            // Draw actual ground line
            draw_rectangle(0.0, ground_screen_y, VIRTUAL_WIDTH, 50.0, Color::new(0.25, 0.2, 0.15, 0.9));
        }
        
        // DANGER OVERLAY - red tint when approaching death zones
        let danger_threshold = 0.12; // 12% from edges = danger zone
        let danger_alpha = if ratio < danger_threshold {
            // Approaching space (top)
            ((danger_threshold - ratio) / danger_threshold * 0.4).min(0.4)
        } else if ratio > (1.0 - danger_threshold) {
            // Approaching ground (bottom)
            ((ratio - (1.0 - danger_threshold)) / danger_threshold * 0.4).min(0.4)
        } else {
            0.0
        };
        
        if danger_alpha > 0.0 {
            draw_rectangle(0.0, 0.0, VIRTUAL_WIDTH, VIRTUAL_HEIGHT, 
                Color::new(1.0, 0.0, 0.0, danger_alpha));
        }
    }

    fn draw_menu(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let cy = VIRTUAL_HEIGHT / 2.0;

        // Title (scaled)
        let title_size = scaled_font(80.0);
        let title_dim = measure_text("FARTCLOUD", None, title_size as u16, 1.0);
        draw_text("FARTCLOUD", cx - title_dim.width / 2.0, cy - scaled(100.0), title_size, Color::new(0.2, 0.8, 0.3, 1.0));
        
        let subtitle_size = scaled_font(30.0);
        let subtitle = "~pff~ Pete pour voler ! ~pff~";
        let subtitle_dim = measure_text(subtitle, None, subtitle_size as u16, 1.0);
        draw_text(subtitle, cx - subtitle_dim.width / 2.0, cy - scaled(40.0), subtitle_size, WHITE);

        // Animated preview
        let preview_y = cy + scaled(30.0) + (get_time() as f32 * 3.0).sin() * scaled(20.0);
        let preview_player = Player { x: cx, y: preview_y, velocity_y: 0.0 };
        preview_player.draw(self.config.player_size * scale(), preview_y, AltitudeZone::Sky, 0.0);

        // Controls (scaled)
        let ctrl_size = scaled_font(24.0);
        let hint_size = scaled_font(18.0);
        draw_text("Controles:", cx - scaled(80.0), cy + scaled(100.0), ctrl_size, WHITE);
        draw_text("Clic/Touch: AU-DESSUS=Monte, EN-DESSOUS=Descend", cx - scaled(200.0), cy + scaled(130.0), hint_size, Color::new(0.8, 0.8, 0.8, 1.0));
        draw_text("Clavier: ESPACE/W=Haut, S=Bas", cx - scaled(130.0), cy + scaled(155.0), hint_size, Color::new(0.8, 0.8, 0.8, 1.0));
        draw_text("* Bonus selon altitude!", cx - scaled(100.0), cy + scaled(180.0), hint_size, GOLD);

        let instr = if self.player_name.is_empty() {
            "ESPACE ou CLIC pour commencer"
        } else {
            &format!("{} - ESPACE/CLIC pour jouer", self.player_name)
        };
        let instr_size = scaled_font(24.0);
        let instr_dim = measure_text(instr, None, instr_size as u16, 1.0);
        draw_text(instr, cx - instr_dim.width / 2.0, cy + scaled(220.0), instr_size, WHITE);

        if self.high_score > 0 {
            let hs_text = format!("Record: {}", self.high_score);
            let hs_size = scaled_font(20.0);
            let hs_dim = measure_text(&hs_text, None, hs_size as u16, 1.0);
            draw_text(&hs_text, cx - hs_dim.width / 2.0, cy + scaled(260.0), hs_size, GOLD);
        }
    }

    fn draw_main_menu(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let s = scale();
        
        // Title area
        let title_size = scaled_font(60.0);
        let title_dim = measure_text("FARTCLOUD", None, title_size as u16, 1.0);
        draw_text("FARTCLOUD", cx - title_dim.width / 2.0, scaled(80.0), title_size, Color::new(0.2, 0.8, 0.3, 1.0));
        
        // Subtitle with player name
        let subtitle = if self.player_name.is_empty() {
            "Bienvenue, Peteur Anonyme!".to_string()
        } else {
            format!("Bienvenue, {}!", self.player_name)
        };
        let sub_size = scaled_font(22.0);
        let sub_dim = measure_text(&subtitle, None, sub_size as u16, 1.0);
        draw_text(&subtitle, cx - sub_dim.width / 2.0, scaled(115.0), sub_size, WHITE);
        
        // Buttons
        let btn_width = scaled(280.0);
        let btn_height = scaled(50.0);
        let btn_x = cx - btn_width / 2.0;
        let btn_start_y = scaled(160.0);
        let btn_spacing = scaled(65.0);
        
        // Button 0: JOUER
        let btn_play = Button::new("JOUER", btn_x, btn_start_y, btn_width, btn_height);
        btn_play.draw(self.selected_button == 0);
        
        // Button 1: LEADERBOARD
        let btn_leaderboard = Button::new("LEADERBOARD", btn_x, btn_start_y + btn_spacing, btn_width, btn_height);
        btn_leaderboard.draw(self.selected_button == 1);
        
        // Button 2: PARTIE PERSO (disabled)
        let btn_custom = Button::disabled("PARTIE PERSO", btn_x, btn_start_y + btn_spacing * 2.0, btn_width, btn_height);
        btn_custom.draw(self.selected_button == 2);
        
        // High score display
        if self.high_score > 0 {
            let hs_text = format!("Ton record: {}", self.high_score);
            let hs_size = scaled_font(18.0);
            let hs_dim = measure_text(&hs_text, None, hs_size as u16, 1.0);
            draw_text(&hs_text, cx - hs_dim.width / 2.0, scaled(420.0), hs_size, GOLD);
        }
        
        // Controls hint
        let hint = "Fleches/WASD + Entree | Souris/Touch";
        let hint_size = scaled_font(14.0);
        let hint_dim = measure_text(hint, None, hint_size as u16, 1.0);
        draw_text(hint, cx - hint_dim.width / 2.0, VIRTUAL_HEIGHT - scaled(20.0), hint_size, GRAY);
        
        // Handle button clicks (in update, but also check for hover state)
        if btn_play.is_activated() || (self.selected_button == 0 && is_key_pressed(KeyCode::Enter)) {
            // Handled in update
        }
        if btn_leaderboard.is_activated() || (self.selected_button == 1 && is_key_pressed(KeyCode::Enter)) {
            // Handled in update
        }
    }

    fn draw_leaderboard_screen(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let s = scale();
        
        // Title
        let title_size = scaled_font(48.0);
        let title_dim = measure_text("LEADERBOARD", None, title_size as u16, 1.0);
        draw_text("LEADERBOARD", cx - title_dim.width / 2.0, scaled(60.0), title_size, GOLD);
        
        // Trophy icon
        draw_text("[TROPHY]", cx - scaled(40.0), scaled(100.0), scaled_font(24.0), GOLD);
        
        // Leaderboard entries
        let entry_start_y = scaled(140.0);
        let entry_spacing = scaled(35.0);
        let entry_size = scaled_font(22.0);
        
        for (i, entry) in self.leaderboard.iter().take(10).enumerate() {
            let rank = i + 1;
            let color = match rank {
                1 => GOLD,
                2 => Color::new(0.75, 0.75, 0.75, 1.0), // Silver
                3 => Color::new(0.8, 0.5, 0.2, 1.0),    // Bronze
                _ => WHITE,
            };
            
            let rank_text = format!("{}.", rank);
            let score_text = format!("{} - {}", entry.name, entry.score);
            
            let y = entry_start_y + i as f32 * entry_spacing;
            draw_text(&rank_text, cx - scaled(120.0), y, entry_size, color);
            draw_text(&score_text, cx - scaled(80.0), y, entry_size, color);
        }
        
        if self.leaderboard.is_empty() {
            draw_text("Aucun score enregistre", cx - scaled(100.0), entry_start_y, entry_size, GRAY);
        }
        
        // Back button
        let btn_width = scaled(200.0);
        let btn_height = scaled(45.0);
        let btn_back = Button::new("RETOUR", cx - btn_width / 2.0, VIRTUAL_HEIGHT - scaled(80.0), btn_width, btn_height);
        btn_back.draw(true);
        
        // Hint
        let hint = "ECHAP ou clic pour revenir";
        let hint_size = scaled_font(14.0);
        let hint_dim = measure_text(hint, None, hint_size as u16, 1.0);
        draw_text(hint, cx - hint_dim.width / 2.0, VIRTUAL_HEIGHT - scaled(20.0), hint_size, GRAY);
    }

    fn draw_custom_game(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let cy = VIRTUAL_HEIGHT / 2.0;
        let s = scale();
        
        // Title
        let title_size = scaled_font(48.0);
        let title_dim = measure_text("PARTIE PERSO", None, title_size as u16, 1.0);
        draw_text("PARTIE PERSO", cx - title_dim.width / 2.0, scaled(60.0), title_size, Color::new(0.6, 0.4, 0.8, 1.0));
        
        // Coming soon message
        let msg1 = "BIENTOT...";
        let msg1_size = scaled_font(36.0);
        let msg1_dim = measure_text(msg1, None, msg1_size as u16, 1.0);
        draw_text(msg1, cx - msg1_dim.width / 2.0, cy - scaled(20.0), msg1_size, GOLD);
        
        let msg2 = "Choix de planete & options";
        let msg2_size = scaled_font(20.0);
        let msg2_dim = measure_text(msg2, None, msg2_size as u16, 1.0);
        draw_text(msg2, cx - msg2_dim.width / 2.0, cy + scaled(20.0), msg2_size, GRAY);
        
        // Placeholder icons
        draw_text("[PLANET]  [SETTINGS]  [ROCKET]", cx - scaled(120.0), cy + scaled(70.0), scaled_font(18.0), Color::new(0.5, 0.5, 0.5, 0.5));
        
        // Back button
        let btn_width = scaled(200.0);
        let btn_height = scaled(45.0);
        let btn_back = Button::new("RETOUR", cx - btn_width / 2.0, VIRTUAL_HEIGHT - scaled(80.0), btn_width, btn_height);
        btn_back.draw(true);
        
        // Hint
        let hint = "ECHAP pour revenir";
        let hint_size = scaled_font(14.0);
        let hint_dim = measure_text(hint, None, hint_size as u16, 1.0);
        draw_text(hint, cx - hint_dim.width / 2.0, VIRTUAL_HEIGHT - scaled(20.0), hint_size, GRAY);
    }

    fn draw_name_input(&self) {
        // On WASM, the HTML overlay handles input, so just show a waiting message
        #[cfg(target_arch = "wasm32")]
        {
            let cx = VIRTUAL_WIDTH / 2.0;
            let cy = VIRTUAL_HEIGHT / 2.0;
            
            // Semi-transparent background
            draw_rectangle(0.0, 0.0, VIRTUAL_WIDTH, VIRTUAL_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.5));
            
            // Message
            let msg = "Utilise le clavier...";
            let msg_size = scaled_font(24.0);
            let msg_dim = measure_text(msg, None, msg_size as u16, 1.0);
            draw_text(msg, cx - msg_dim.width / 2.0, cy, msg_size, WHITE);
        }
        
        // On desktop, show the full input UI
        #[cfg(not(target_arch = "wasm32"))]
        {
            let cx = VIRTUAL_WIDTH / 2.0;
            let cy = VIRTUAL_HEIGHT / 2.0;

            // Title
            let title_size = scaled_font(30.0);
            let title = "Entre ton pseudo:";
            let title_dim = measure_text(title, None, title_size as u16, 1.0);
            draw_text(title, cx - title_dim.width / 2.0, cy - scaled(50.0), title_size, WHITE);
            
            // Input box
            let box_width = scaled(280.0);
            let box_height = scaled(55.0);
            let box_x = cx - box_width / 2.0;
            let box_y = cy - scaled(10.0);
            draw_rectangle(box_x, box_y, box_width, box_height, Color::new(1.0, 1.0, 1.0, 0.9));
            draw_rectangle_lines(box_x, box_y, box_width, box_height, 3.0, Color::new(0.2, 0.6, 0.2, 1.0));
            
            // Input text
            let display_text = if self.name_input.is_empty() { "FartKing42" } else { &self.name_input };
            let text_color = if self.name_input.is_empty() { GRAY } else { BLACK };
            let text_size = scaled_font(28.0);
            let text_dim = measure_text(display_text, None, text_size as u16, 1.0);
            draw_text(display_text, cx - text_dim.width / 2.0, box_y + box_height * 0.65, text_size, text_color);
            
            // Blinking cursor
            if self.name_input.len() < 12 && (get_time() * 2.0) as i32 % 2 == 0 {
                let cursor_x = cx - text_dim.width / 2.0 + text_dim.width + scaled(4.0);
                draw_line(cursor_x, box_y + scaled(10.0), cursor_x, box_y + box_height - scaled(10.0), 2.0, BLACK);
            }

            // Hint
            let hint = "ENTREE pour confirmer";
            let hint_size = scaled_font(20.0);
            let hint_dim = measure_text(hint, None, hint_size as u16, 1.0);
            draw_text(hint, cx - hint_dim.width / 2.0, cy + scaled(80.0), hint_size, WHITE);
        }
    }

    fn draw_gameplay(&self, offset: Vec2, player_scale: f32, sounds: &SoundRegistry) {
        // Draw obstacles
        for obs in &self.obstacles {
            obs.draw(self.camera_y - offset.y);
        }

        // Draw particles
        for particle in &self.particles {
            particle.draw(self.camera_y - offset.y);
        }

        // Draw floating texts
        for ft in &self.floating_texts {
            ft.draw(self.camera_y);
        }

        // Draw player (with death animation scale)
        if player_scale > 0.0 {
            let player_screen_y = self.player.y - self.camera_y + VIRTUAL_HEIGHT / 2.0 + offset.y;
            self.draw_player_with_scale(player_screen_y, player_scale);
        }

        // Draw click effects (on top of everything except HUD)
        for effect in &self.click_effects {
            effect.draw();
        }

        // HUD
        self.draw_hud(sounds);
    }

    fn draw_hud(&self, sounds: &SoundRegistry) {
        // Mute indicator (top right)
        let mute_text = if sounds.is_muted { "[X] M" } else { "[*] M" };
        draw_text(mute_text, VIRTUAL_WIDTH - 70.0, 25.0, 18.0, 
            if sounds.is_muted { Color::new(1.0, 0.3, 0.3, 0.8) } else { Color::new(0.5, 1.0, 0.5, 0.8) });
        
        // Zone indicator
        draw_text(self.current_zone.zone_name(), 20.0, 35.0, 28.0, WHITE);
        
        // Score (now time-based) - removed emoji
        let score_text = format!("Score: {:.0}  Pets: {}", self.score, self.fart_count);
        draw_text(&score_text, 20.0, 65.0, 24.0, WHITE);
        
        // Difficulty & Time
        let time_text = format!("Temps: {:.0}s  Niveau: {}", self.play_time, self.difficulty_level);
        draw_text(&time_text, 20.0, 90.0, 18.0, Color::new(0.8, 0.8, 0.8, 1.0));
        
        // Combo display: show count and pet boost percentage
        if self.combo_count >= 3 {
            let pet_boost = ((self.combo_multiplier - 1.0) * 100.0) as i32;
            let combo_text = format!("COMBO {}x! (+{}% pet)", self.combo_count, pet_boost);
            let pulse = ((get_time() * 10.0).sin() * 0.2 + 0.8) as f32;
            draw_text(&combo_text, 20.0, 120.0, 28.0, Color::new(1.0, 0.5 * pulse, 0.0, 1.0));
        }
        
        // Altitude bar (right side)
        let bar_x = VIRTUAL_WIDTH - 30.0;
        let bar_height = VIRTUAL_HEIGHT - 100.0;
        let bar_y = 50.0;
        
        // Background
        draw_rectangle(bar_x - 10.0, bar_y, 20.0, bar_height, Color::new(0.0, 0.0, 0.0, 0.3));
        
        // Zone colors with danger indicators
        let zone_height = bar_height / 4.0;
        // Space zone (danger - explode)
        draw_rectangle(bar_x - 8.0, bar_y + 2.0, 16.0, zone_height - 4.0, Color::new(0.3, 0.1, 0.1, 0.7));
        draw_text("X", bar_x - 10.0, bar_y + 20.0, 16.0, RED);
        // High sky
        draw_rectangle(bar_x - 8.0, bar_y + zone_height, 16.0, zone_height - 4.0, Color::new(0.2, 0.4, 0.7, 0.7));
        // Sky (safe)
        draw_rectangle(bar_x - 8.0, bar_y + zone_height * 2.0, 16.0, zone_height - 4.0, Color::new(0.4, 0.7, 0.9, 0.7));
        // Ground zone (danger - splat)
        draw_rectangle(bar_x - 8.0, bar_y + zone_height * 3.0, 16.0, zone_height - 4.0, Color::new(0.3, 0.1, 0.1, 0.7));
        draw_text("X", bar_x - 10.0, bar_y + zone_height * 4.0 - 10.0, 16.0, RED);
        
        // Player position indicator
        let player_ratio = self.player.y / self.config.world_height;
        let indicator_y = bar_y + player_ratio * bar_height;
        draw_circle(bar_x, indicator_y, 8.0, Color::new(0.4, 1.0, 0.4, 1.0));
        draw_circle(bar_x, indicator_y, 5.0, WHITE);
        
        // Gravity indicator (progressive based on position)
        let gravity_mult = calculate_gravity_multiplier(self.player.y, self.config.world_height);
        let grav_text = format!("Gravité: {:.0}%", gravity_mult * 100.0);
        draw_text(&grav_text, VIRTUAL_WIDTH - 130.0, VIRTUAL_HEIGHT - 20.0, 16.0, 
            if gravity_mult < 0.9 { Color::new(0.5, 0.8, 1.0, 1.0) } 
            else if gravity_mult > 1.1 { Color::new(1.0, 0.6, 0.4, 1.0) }
            else { WHITE });
        
        // DANGER WARNING - RED FUNKY ALERTS with increasing intensity
        let ratio = self.player.y / self.config.world_height;
        let cx = VIRTUAL_WIDTH / 2.0;
        let time = get_time();
        
        // Check if we're in a danger zone
        let (in_danger, urgency, is_space) = if ratio < 0.25 {
            (true, 1.0 - (ratio / 0.25), true)
        } else if ratio > 0.75 {
            (true, (ratio - 0.75) / 0.25, false)
        } else {
            (false, 0.0, false)
        };
        
        if in_danger {
            // Beep frequency increases with urgency: 0.5s -> 0.05s
            let beep_interval = 0.5 - urgency * 0.45;
            let should_beep = (time - self.last_beep_time) > beep_interval as f64;
            
            // Funky pulsing effect
            let pulse = (time * (3.0 + urgency as f64 * 12.0)).sin() as f32;
            let shake_x = if urgency > 0.5 { (time * 25.0).sin() as f32 * urgency * 5.0 } else { 0.0 };
            let shake_y = if urgency > 0.7 { (time * 30.0).cos() as f32 * urgency * 3.0 } else { 0.0 };
            
            // Position: TOP for space, BOTTOM for ground
            let box_y = if is_space { 30.0 + shake_y } else { VIRTUAL_HEIGHT - 90.0 + shake_y };
            
            // Box size pulses with urgency
            let base_width = 300.0 + urgency * 50.0;
            let base_height = 60.0 + urgency * 20.0;
            let pulse_scale = 1.0 + pulse * 0.05 * urgency;
            let box_width = base_width * pulse_scale;
            let box_height = base_height * pulse_scale;
            
            // RED intensity increases with urgency
            let red_intensity = 0.4 + urgency * 0.6;
            let flash_boost = self.beep_flash * 0.3;
            let bg_alpha = (0.6 + urgency * 0.3 + flash_boost).min(1.0);
            
            // Background - dark red getting brighter
            draw_rectangle(
                cx - box_width/2.0 + shake_x, box_y, box_width, box_height,
                Color::new(red_intensity * 0.3 + flash_boost, 0.0, 0.0, bg_alpha)
            );
            
            // Multiple border layers for funky effect
            let border_alpha = 0.7 + urgency * 0.3;
            for i in 0..3 {
                let offset = i as f32 * 2.0;
                let border_pulse = ((time * (4.0 + i as f64)).sin() as f32 + 1.0) * 0.5;
                draw_rectangle_lines(
                    cx - box_width/2.0 + shake_x - offset, box_y - offset,
                    box_width + offset * 2.0, box_height + offset * 2.0,
                    2.0 + urgency * 2.0,
                    Color::new(1.0, border_pulse * 0.3, 0.0, border_alpha * (1.0 - i as f32 * 0.2))
                );
            }
            
            // Inner glow on beep
            if self.beep_flash > 0.3 {
                draw_rectangle(
                    cx - box_width/2.0 + shake_x + 5.0, box_y + 5.0,
                    box_width - 10.0, box_height - 10.0,
                    Color::new(1.0, 0.2, 0.0, self.beep_flash * 0.5)
                );
            }
            
            // Text with shake
            let text = if is_space { "!!! ESPACE !!!" } else { "!!! SOL !!!" };
            let arrow = if is_space { "vvv" } else { "^^^" };
            let text_y = box_y + box_height / 2.0 + 8.0;
            let text_size = 28.0 + urgency * 8.0;
            let text_alpha = 0.8 + self.beep_flash * 0.2;
            
            // Shadow
            draw_text(text, cx - 75.0 + shake_x + 2.0, text_y + 2.0, text_size, Color::new(0.0, 0.0, 0.0, 0.5));
            // Main text
            draw_text(text, cx - 75.0 + shake_x, text_y, text_size, Color::new(1.0, 1.0, 1.0, text_alpha));
            
            // Arrows on sides
            draw_text(arrow, cx - 130.0 + shake_x, text_y, 32.0, Color::new(1.0, 0.3, 0.0, text_alpha));
            draw_text(arrow, cx + 95.0 + shake_x, text_y, 32.0, Color::new(1.0, 0.3, 0.0, text_alpha));
            
            // Urgency bar at edge of screen
            let bar_height = VIRTUAL_HEIGHT * urgency * 0.3;
            if is_space {
                draw_rectangle(0.0, 0.0, VIRTUAL_WIDTH, bar_height, Color::new(1.0, 0.0, 0.0, urgency * 0.2));
            } else {
                draw_rectangle(0.0, VIRTUAL_HEIGHT - bar_height, VIRTUAL_WIDTH, bar_height, Color::new(1.0, 0.0, 0.0, urgency * 0.2));
            }
            
            // "BEEP" visual indicator (flashing circles on sides)
            if self.beep_flash > 0.1 {
                let circle_y = if is_space { 60.0 } else { VIRTUAL_HEIGHT - 60.0 };
                draw_circle(50.0, circle_y, 15.0 * self.beep_flash, Color::new(1.0, 0.0, 0.0, self.beep_flash));
                draw_circle(VIRTUAL_WIDTH - 50.0, circle_y, 15.0 * self.beep_flash, Color::new(1.0, 0.0, 0.0, self.beep_flash));
            }
        }
    }
    
    fn draw_player_with_scale(&self, screen_y: f32, scale: f32) {
        let size = self.config.player_size;
        let zone = self.current_zone;
        let velocity = self.player.velocity_y;
        let x = self.player.x;
        
        // Death animation transforms
        let (scale_x, scale_y) = match self.death_type {
            DeathType::Splat => (scale * 1.5, 1.0 / scale.max(0.3)), // Squish flat
            DeathType::Explode => (scale, scale), // Inflate uniformly
            _ => (1.0, 1.0),
        };
        
        let effective_size = size * scale_x.max(scale_y).min(3.0);
        
        // Color based on zone (or death state)
        let (base_color, highlight) = if self.death_type != DeathType::None {
            (Color::new(1.0, 0.5, 0.3, 1.0), Color::new(1.0, 0.7, 0.5, 1.0))
        } else {
            match zone {
                AltitudeZone::Space => (
                    Color::new(0.5, 0.6, 1.0, 1.0),
                    Color::new(0.7, 0.8, 1.0, 1.0),
                ),
                AltitudeZone::HighSky => (
                    Color::new(0.4, 0.8, 0.6, 1.0),
                    Color::new(0.5, 0.9, 0.7, 1.0),
                ),
                AltitudeZone::Sky => (
                    Color::new(0.4, 0.9, 0.4, 1.0),
                    Color::new(0.5, 1.0, 0.5, 1.0),
                ),
                AltitudeZone::Ground => (
                    Color::new(0.7, 0.6, 0.3, 1.0),
                    Color::new(0.8, 0.7, 0.4, 1.0),
                ),
            }
        };
        
        // Cloud body (stretched for death animation)
        draw_ellipse(x, screen_y, effective_size * 0.9 * scale_x, effective_size * 0.9 * scale_y, 0.0, base_color);
        draw_ellipse(x - effective_size * 0.5, screen_y + effective_size * 0.15, effective_size * 0.5 * scale_x, effective_size * 0.4 * scale_y, 0.0, base_color);
        draw_ellipse(x + effective_size * 0.5, screen_y + effective_size * 0.15, effective_size * 0.5 * scale_x, effective_size * 0.45 * scale_y, 0.0, base_color);
        draw_ellipse(x - effective_size * 0.2, screen_y - effective_size * 0.3, effective_size * 0.35 * scale_x, effective_size * 0.35 * scale_y, 0.0, highlight);
        
        // Only draw face if not exploded
        if self.death_type != DeathType::Explode || scale > 0.5 {
            // Eyes
            let eye_y = screen_y - effective_size * 0.1 * scale_y;
            let eye_scale = if self.death_type != DeathType::None { 1.5 } else if velocity.abs() > 200.0 { 1.3 } else { 1.0 };
            draw_circle(x - effective_size * 0.2, eye_y, effective_size * 0.12 * eye_scale, WHITE);
            draw_circle(x + effective_size * 0.2, eye_y, effective_size * 0.12 * eye_scale, WHITE);
            
            // Pupils (X eyes when dying)
            if self.death_type != DeathType::None {
                let eye_size = effective_size * 0.08;
                // X for left eye
                draw_line(x - effective_size * 0.2 - eye_size, eye_y - eye_size, x - effective_size * 0.2 + eye_size, eye_y + eye_size, 2.0, BLACK);
                draw_line(x - effective_size * 0.2 - eye_size, eye_y + eye_size, x - effective_size * 0.2 + eye_size, eye_y - eye_size, 2.0, BLACK);
                // X for right eye
                draw_line(x + effective_size * 0.2 - eye_size, eye_y - eye_size, x + effective_size * 0.2 + eye_size, eye_y + eye_size, 2.0, BLACK);
                draw_line(x + effective_size * 0.2 - eye_size, eye_y + eye_size, x + effective_size * 0.2 + eye_size, eye_y - eye_size, 2.0, BLACK);
            } else {
                let pupil_offset_y = (velocity / 1000.0).clamp(-0.05, 0.05) * effective_size;
                draw_circle(x - effective_size * 0.2, eye_y + pupil_offset_y, effective_size * 0.06, BLACK);
                draw_circle(x + effective_size * 0.2, eye_y + pupil_offset_y, effective_size * 0.06, BLACK);
            }
            
            // Mouth
            if self.death_type != DeathType::None {
                // Dead mouth - wavy line
                draw_line(x - effective_size * 0.15, screen_y + effective_size * 0.25, x + effective_size * 0.15, screen_y + effective_size * 0.25, 3.0, Color::new(0.3, 0.2, 0.2, 1.0));
            } else if velocity < -150.0 {
                draw_circle(x, screen_y + effective_size * 0.2, effective_size * 0.12, Color::new(0.3, 0.2, 0.2, 1.0));
            } else {
                draw_circle(x, screen_y + effective_size * 0.25, effective_size * 0.06, Color::new(0.3, 0.6, 0.3, 1.0));
            }
        }
    }

    fn draw_game_over(&self) {
        let cx = VIRTUAL_WIDTH / 2.0;
        let cy = VIRTUAL_HEIGHT / 2.0;

        // Dark overlay
        draw_rectangle(0.0, 0.0, VIRTUAL_WIDTH, VIRTUAL_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.7));
        
        // Death type specific message
        let death_msg = match self.death_type {
            DeathType::Splat => "Tu t'es ecrase au sol!",
            DeathType::Explode => "Tu as explose dans l'espace!",
            DeathType::Cloud => "Tu as percute un nuage!",
            DeathType::None => "Game Over!",
        };

        // GAME OVER title
        let title_size = scaled_font(60.0);
        let title = "GAME OVER";
        let title_dim = measure_text(title, None, title_size as u16, 1.0);
        draw_text(title, cx - title_dim.width / 2.0, cy - scaled(120.0), title_size, RED);
        
        // Death message
        let death_size = scaled_font(20.0);
        let death_dim = measure_text(death_msg, None, death_size as u16, 1.0);
        draw_text(death_msg, cx - death_dim.width / 2.0, cy - scaled(80.0), death_size, Color::new(0.9, 0.7, 0.5, 1.0));

        // Score
        let total_score = self.score as u32;
        let score_text = format!("Score: {}", total_score);
        let score_size = scaled_font(32.0);
        let score_dim = measure_text(&score_text, None, score_size as u16, 1.0);
        draw_text(&score_text, cx - score_dim.width / 2.0, cy - scaled(40.0), score_size, WHITE);
        
        // Stats
        let stats_text = format!("Temps: {:.1}s  |  Pets: {}", self.play_time, self.fart_count);
        let stats_size = scaled_font(20.0);
        let stats_dim = measure_text(&stats_text, None, stats_size as u16, 1.0);
        draw_text(&stats_text, cx - stats_dim.width / 2.0, cy, stats_size, Color::new(0.8, 0.8, 0.8, 1.0));
        
        let level_text = format!("Niveau max: {}", self.difficulty_level);
        let level_size = scaled_font(18.0);
        let level_dim = measure_text(&level_text, None, level_size as u16, 1.0);
        draw_text(&level_text, cx - level_dim.width / 2.0, cy + scaled(25.0), level_size, Color::new(0.8, 0.8, 0.8, 1.0));

        // New record
        if total_score == self.high_score && total_score > 0 {
            let record_text = "*** NOUVEAU RECORD! ***";
            let record_size = scaled_font(28.0);
            let record_dim = measure_text(record_text, None, record_size as u16, 1.0);
            draw_text(record_text, cx - record_dim.width / 2.0, cy + scaled(60.0), record_size, GOLD);
        }

        // Mini leaderboard
        let lb_title = "=== Leaderboard ===";
        let lb_title_size = scaled_font(24.0);
        let lb_title_dim = measure_text(lb_title, None, lb_title_size as u16, 1.0);
        draw_text(lb_title, cx - lb_title_dim.width / 2.0, cy + scaled(100.0), lb_title_size, GOLD);
        
        // Show loading indicator if still fetching
        if is_leaderboard_loading() || self.leaderboard.is_empty() {
            let loading_size = scaled_font(18.0);
            let dots = match ((get_time() * 3.0) as i32) % 4 {
                0 => "Chargement",
                1 => "Chargement.",
                2 => "Chargement..",
                _ => "Chargement...",
            };
            let dots_dim = measure_text(dots, None, loading_size as u16, 1.0);
            draw_text(dots, cx - dots_dim.width / 2.0, cy + scaled(140.0), loading_size, GRAY);
        } else {
            // Find player's position in leaderboard
            let player_score = self.score as u32;
            let player_idx = self.leaderboard.iter()
                .position(|e| e.name == self.player_name && e.score == player_score);
            
            // If not found by exact match, find by score (first occurrence)
            let player_rank = player_idx.unwrap_or_else(|| {
                self.leaderboard.iter()
                    .position(|e| e.score <= player_score)
                    .unwrap_or(self.leaderboard.len())
            });
            
            let in_top_5 = player_rank < 5;
            let entry_size = scaled_font(16.0);
            let mut y_offset = scaled(130.0);
            
            // Helper to draw an entry
            let draw_entry = |rank: usize, entry: &LeaderboardEntry, y: f32, is_player: bool| {
                let entry_text = format!("#{:<3} {:<12} {:>5}", rank + 1, 
                    if entry.name.len() > 12 { &entry.name[..12] } else { &entry.name }, 
                    entry.score);
                let entry_dim = measure_text(&entry_text, None, entry_size as u16, 1.0);
                
                if is_player {
                    // Gold background for player
                    draw_rectangle(
                        cx - entry_dim.width / 2.0 - scaled(8.0),
                        y - scaled(14.0),
                        entry_dim.width + scaled(16.0),
                        scaled(20.0),
                        Color::new(1.0, 0.84, 0.0, 0.3)
                    );
                }
                
                let color = if is_player { GOLD } else { WHITE };
                draw_text(&entry_text, cx - entry_dim.width / 2.0, y, entry_size, color);
            };
            
            // Draw TOP 5
            for i in 0..5.min(self.leaderboard.len()) {
                let is_player = i == player_rank;
                draw_entry(i, &self.leaderboard[i], cy + y_offset, is_player);
                y_offset += scaled(22.0);
            }
            
            // If player is NOT in top 5, show separator and player context
            if !in_top_5 && player_rank < self.leaderboard.len() {
                // Separator
                y_offset += scaled(5.0);
                let sep = "· · ·";
                let sep_dim = measure_text(sep, None, entry_size as u16, 1.0);
                draw_text(sep, cx - sep_dim.width / 2.0, cy + y_offset, entry_size, GRAY);
                y_offset += scaled(20.0);
                
                // Show 2 above, player, 2 below
                let start = if player_rank >= 2 { player_rank - 2 } else { 0 };
                let end = (player_rank + 3).min(self.leaderboard.len());
                
                // Skip entries already shown in top 5
                for i in start..end {
                    if i < 5 { continue; } // Already shown in top 5
                    let is_player = i == player_rank;
                    draw_entry(i, &self.leaderboard[i], cy + y_offset, is_player);
                    y_offset += scaled(22.0);
                }
            }
        }

        // Instructions
        let hint = "ESPACE/CLIC = Rejouer | ECHAP = Menu";
        let hint_size = scaled_font(20.0);
        let hint_dim = measure_text(hint, None, hint_size as u16, 1.0);
        draw_text(hint, cx - hint_dim.width / 2.0, VIRTUAL_HEIGHT - scaled(40.0), hint_size, WHITE);
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

fn window_conf() -> Conf {
    Conf {
        window_title: "FartCloud - Pete pour voler!".to_owned(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut config: GameConfig = load_string("assets/config.json")
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Apply platform config override (partial merge)
    if let Some(override_json) = get_platform_config_override() {
        // Parse the override as a serde_json::Value and merge fields
        if let Ok(override_val) = serde_json::from_str::<serde_json::Value>(&override_json) {
            if let Ok(mut config_val) = serde_json::to_value(&config) {
                if let (Some(base), Some(overrides)) = (config_val.as_object_mut(), override_val.as_object()) {
                    for (key, val) in overrides {
                        base.insert(key.clone(), val.clone());
                    }
                    if let Ok(merged) = serde_json::from_value(config_val) {
                        config = merged;
                    }
                }
            }
        }
    }

    // Load sprites
    let mut sprites = SpriteRegistry::new();
    sprites.load_sprites().await;
    
    // Load sounds
    let mut sounds = SoundRegistry::new(config.master_volume, config.sfx_volume);
    sounds.load_sounds().await;

    let mut game = Game::new(config);

    loop {
        let dt = get_frame_time();
        
        // Update sound system (ducking timer)
        sounds.update(dt);
        
        // Handle mute toggle
        if is_key_pressed(KeyCode::M) {
            sounds.toggle_mute();
        }
        
        game.update(dt, &mut sounds);
        
        // Play pending sounds
        for action in game.pending_sounds.drain(..) {
            sounds.play(action);
        }
        
        // Check for portrait mode - draw overlay directly to screen
        if is_portrait() {
            set_default_camera();
            clear_background(BLACK);
            draw_rotate_overlay();
            next_frame().await;
            continue;
        }
        
        // Clear screen (CSS handles black letterbox bars)
        clear_background(BLACK);
        
        // Calculate letterbox parameters for camera setup
        let (scale, offset_x, offset_y, _game_w, _game_h) = letterbox_params();
        
        // Set up camera for 16:9 letterboxing (WebGL1 compatible - no render target)
        // Camera maps virtual coords (0..VIRTUAL_WIDTH, 0..VIRTUAL_HEIGHT) to the game area on screen
        set_camera(&Camera2D {
            zoom: vec2(2.0 * scale / screen_width(), 2.0 * scale / screen_height()),
            target: vec2(VIRTUAL_WIDTH / 2.0, VIRTUAL_HEIGHT / 2.0),
            offset: vec2(
                (offset_x + _game_w / 2.0) / screen_width() * 2.0 - 1.0,
                1.0 - (offset_y + _game_h / 2.0) / screen_height() * 2.0
            ),
            ..Default::default()
        });
        
        game.draw_game(&sprites, &sounds);
        
        // Switch back to screen coords and draw letterbox bars on top to clip overflow
        set_default_camera();
        draw_letterbox_bars();
        
        next_frame().await;
    }
}
