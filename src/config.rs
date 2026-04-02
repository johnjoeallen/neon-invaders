use macroquad::prelude::*;

pub const WINDOW_WIDTH: f32 = 1920.0;
pub const WINDOW_HEIGHT: f32 = 1080.0;

pub const PLAYER_Y_OFFSET: f32 = 92.0;
pub const PLAYER_SPEED: f32 = 760.0;
pub const PLAYER_WIDTH: f32 = 82.0;
pub const PLAYER_HEIGHT: f32 = 28.0;
pub const PLAYER_COOLDOWN: f32 = 0.26;
pub const PLAYER_BOMB_COOLDOWN: f32 = 0.55;

pub const PLAYER_SHOT_SPEED: f32 = 920.0;
pub const PLAYER_BOMB_SPEED: f32 = 700.0;
pub const PLAYER_BOMB_RADIUS: f32 = 150.0;
pub const ENEMY_SHOT_SPEED: f32 = 380.0;
pub const ENEMY_SHOT_MAX_X_SPEED: f32 = 220.0;
pub const ENEMY_AIM_BIAS: f32 = 0.92;
pub const SHOT_WIDTH: f32 = 8.0;
pub const PLAYER_SHOT_HEIGHT: f32 = 26.0;
pub const PLAYER_BOMB_WIDTH: f32 = 16.0;
pub const PLAYER_BOMB_HEIGHT: f32 = 30.0;
pub const ENEMY_SHOT_HEIGHT: f32 = 20.0;
pub const BOMB_SPEED: f32 = 300.0;
pub const BOMB_WIDTH: f32 = 18.0;
pub const BOMB_HEIGHT: f32 = 28.0;
pub const BOMB_DETONATION_Y_OFFSET: f32 = 18.0;
pub const BOMB_WAVE_SPEED: f32 = 760.0;
pub const BOMB_WAVE_MAX_RADIUS: f32 = 340.0;
pub const BOMB_FIRE_CHANCE: f32 = 0.18;
pub const BOMB_MIN_INTERVAL_SCALE: f32 = 1.7;

pub const ALIEN_ROWS: usize = 5;
pub const ALIEN_COLS: usize = 10;
pub const ALIEN_SPACING_X: f32 = 96.0;
pub const ALIEN_SPACING_Y: f32 = 74.0;
pub const ALIEN_SIZE: Vec2 = Vec2::from_array([58.0, 42.0]);
pub const ALIEN_START_X: f32 = 528.0;
pub const ALIEN_START_Y: f32 = 170.0;
pub const ALIEN_BASE_SPEED: f32 = 82.0;
pub const ALIEN_STEP_DOWN: f32 = 28.0;
pub const ALIEN_STEP_DOWN_PASS_BONUS: f32 = 1.5;
pub const ALIEN_MARCH_DISTANCE: f32 = 54.0;
pub const ALIEN_DIVE_BASE_INTERVAL: f32 = 8.0;
pub const ALIEN_DIVE_MIN_INTERVAL: f32 = 3.8;
pub const ALIEN_DIVE_SPEED: f32 = 340.0;
pub const ALIEN_DIVE_TURN_RATE: f32 = 2.8;
pub const ALIEN_DIVE_SPIN_SPEED: f32 = 9.5;
pub const ENEMY_FIRE_BASE_INTERVAL: f32 = 1.25;
pub const ENEMY_FIRE_MIN_INTERVAL: f32 = 0.38;

pub const BUNKER_COUNT: usize = 4;
pub const BUNKER_GRID_W: usize = 10;
pub const BUNKER_GRID_H: usize = 6;
pub const BUNKER_CELL: f32 = 14.0;
pub const BUNKER_Y: f32 = WINDOW_HEIGHT - 260.0;

pub const PLAYER_ZONE_Y: f32 = WINDOW_HEIGHT - 165.0;
pub const INVADER_REACH_MARGIN: f32 = 28.0;

pub const PARTICLE_CAP: usize = 700;

pub const TITLE_FADE_TIME: f32 = 0.45;
pub const WAVE_INTRO_TIME: f32 = 1.2;
pub const WAVE_CLEAR_TIME: f32 = 1.4;
pub const GAME_OVER_DELAY: f32 = 0.8;
pub const BOMB_REWARD_WINDOW: f32 = 5.0;
pub const BOMB_REWARD_KILLS: u32 = 4;

pub const STAR_LAYER_COUNT: usize = 3;
pub const STARS_PER_LAYER: usize = 48;

pub const BG_TOP: Color = Color::from_rgba(3, 4, 12, 255);
pub const BG_BOTTOM: Color = Color::from_rgba(8, 10, 26, 255);
pub const BG_MID: Color = Color::from_rgba(13, 17, 42, 255);
pub const HUD_COLOR: Color = Color::from_rgba(213, 240, 255, 255);
pub const ACCENT_A: Color = Color::from_rgba(76, 236, 255, 255);
pub const ACCENT_B: Color = Color::from_rgba(255, 90, 163, 255);
pub const ACCENT_C: Color = Color::from_rgba(255, 214, 92, 255);
pub const PLAYER_COLOR: Color = Color::from_rgba(108, 255, 196, 255);
pub const PLAYER_GLOW: Color = Color::from_rgba(108, 255, 196, 70);
pub const BUNKER_COLOR: Color = Color::from_rgba(104, 255, 133, 255);
pub const ENEMY_SHOT_COLOR: Color = Color::from_rgba(255, 104, 141, 255);
pub const PLAYER_SHOT_COLOR: Color = Color::from_rgba(98, 247, 255, 255);
pub const PLAYER_BOMB_COLOR: Color = Color::from_rgba(255, 214, 92, 255);

pub const ALIEN_ROW_COLORS: [Color; ALIEN_ROWS] = [
    Color::from_rgba(255, 226, 104, 255),
    Color::from_rgba(255, 118, 88, 255),
    Color::from_rgba(255, 76, 120, 255),
    Color::from_rgba(115, 208, 255, 255),
    Color::from_rgba(132, 255, 184, 255),
];
