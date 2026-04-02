use crate::audio::SoundBank;
use crate::config;
use macroquad::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScreenState {
    Title,
    WaveIntro,
    Playing,
    Paused,
    WaveClear,
    GameOver,
}

#[derive(Clone, Copy)]
struct Player {
    x: f32,
    cooldown: f32,
    hit_flash: f32,
    bombs: u32,
}

#[derive(Clone, Copy)]
struct Shot {
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    from_player: bool,
    kind: ShotKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShotKind {
    Bolt,
    EnemyBomb,
    PlayerBomb,
}

#[derive(Clone, Copy)]
struct Alien {
    row: usize,
    col: usize,
    alive: bool,
    frame: bool,
    fire_flash: f32,
    diving: bool,
    dive_pos: Vec2,
    dive_vel: Vec2,
    dive_angle: f32,
}

#[derive(Clone)]
struct Bunker {
    origin: Vec2,
    cells: [[u8; config::BUNKER_GRID_W]; config::BUNKER_GRID_H],
}

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

#[derive(Clone, Copy)]
struct BlastWave {
    center: Vec2,
    radius: f32,
    hit_player: bool,
}

#[derive(Clone, Copy)]
struct Star {
    pos: Vec2,
    speed: f32,
    radius: f32,
    alpha: f32,
}

pub struct GameApp {
    screen: ScreenState,
    state_timer: f32,
    player: Player,
    shots: Vec<Shot>,
    aliens: Vec<Alien>,
    formation_x: f32,
    formation_y: f32,
    alien_dir: f32,
    enemy_fire_timer: f32,
    bunkers: Vec<Bunker>,
    particles: Vec<Particle>,
    blast_waves: Vec<BlastWave>,
    stars: Vec<Vec<Star>>,
    score: u32,
    high_score: u32,
    lives: u32,
    wave: u32,
    kill_window_timer: f32,
    kill_window_kills: u32,
    screen_shake: f32,
    shield_ring_timer: f32,
    rapid_fire_timer: f32,
    pending_restart: bool,
    march_progress: f32,
    march_step: u32,
    pass_count: u32,
    dive_timer: f32,
    sounds: Option<SoundBank>,
}

impl GameApp {
    pub async fn new() -> Self {
        let mut app = Self {
            screen: ScreenState::Title,
            state_timer: 0.0,
            player: Player {
                x: config::WINDOW_WIDTH * 0.5,
                cooldown: 0.0,
                hit_flash: 0.0,
                bombs: 0,
            },
            shots: Vec::new(),
            aliens: Vec::new(),
            formation_x: config::ALIEN_START_X,
            formation_y: config::ALIEN_START_Y,
            alien_dir: 1.0,
            enemy_fire_timer: config::ENEMY_FIRE_BASE_INTERVAL,
            bunkers: Vec::new(),
            particles: Vec::with_capacity(config::PARTICLE_CAP),
            blast_waves: Vec::new(),
            stars: Vec::new(),
            score: 0,
            high_score: load_high_score(),
            lives: 3,
            wave: 1,
            kill_window_timer: config::BOMB_REWARD_WINDOW,
            kill_window_kills: 0,
            screen_shake: 0.0,
            shield_ring_timer: 0.0,
            rapid_fire_timer: 0.0,
            pending_restart: false,
            march_progress: 0.0,
            march_step: 0,
            pass_count: 0,
            dive_timer: config::ALIEN_DIVE_BASE_INTERVAL,
            sounds: SoundBank::load().await,
        };
        app.reset_for_new_run();
        app.screen = ScreenState::Title;
        app
    }

    pub fn update(&mut self, dt: f32) {
        match self.screen {
            ScreenState::Paused => self.update_paused(),
            ScreenState::Title => self.update_title(dt),
            ScreenState::WaveIntro => self.update_wave_intro(dt),
            ScreenState::Playing => self.update_playing(dt),
            ScreenState::WaveClear => self.update_wave_clear(dt),
            ScreenState::GameOver => self.update_game_over(dt),
        }

        if self.score > self.high_score {
            self.high_score = self.score;
            save_high_score(self.high_score);
        }
    }

    pub fn draw(&self) {
        self.draw_background();

        let shake = if self.screen_shake > 0.0 {
            vec2(
                rand::gen_range(-self.screen_shake, self.screen_shake),
                rand::gen_range(-self.screen_shake, self.screen_shake),
            )
        } else {
            Vec2::ZERO
        };

        self.draw_playfield(shake);
        self.draw_hud();
        self.draw_overlay();
    }

    fn update_title(&mut self, dt: f32) {
        self.state_timer += dt;
        self.update_stars(dt);
        self.update_particles(dt);
        self.update_blast_waves(dt);
        self.screen_shake = (self.screen_shake - dt * 18.0).max(0.0);
        self.shield_ring_timer = (self.shield_ring_timer - dt).max(0.0);
        self.rapid_fire_timer = (self.rapid_fire_timer - dt).max(0.0);
        self.player.cooldown = (self.player.cooldown - dt).max(0.0);
        self.player.hit_flash = (self.player.hit_flash - dt * 3.0).max(0.0);
        if is_key_pressed(KeyCode::Escape) {
            process::exit(0);
        }
        self.idle_aliens(dt);
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            self.pending_restart = false;
            self.screen = ScreenState::WaveIntro;
            self.state_timer = 0.0;
        }
    }

    fn update_wave_intro(&mut self, dt: f32) {
        self.state_timer += dt;
        self.update_stars(dt);
        self.update_particles(dt);
        self.update_blast_waves(dt);
        self.screen_shake = (self.screen_shake - dt * 18.0).max(0.0);
        self.shield_ring_timer = (self.shield_ring_timer - dt).max(0.0);
        self.rapid_fire_timer = (self.rapid_fire_timer - dt).max(0.0);
        self.player.cooldown = (self.player.cooldown - dt).max(0.0);
        self.player.hit_flash = (self.player.hit_flash - dt * 3.0).max(0.0);
        self.idle_aliens(dt);
        if self.state_timer >= config::WAVE_INTRO_TIME {
            self.screen = ScreenState::Playing;
            self.state_timer = 0.0;
        }
    }

    fn update_wave_clear(&mut self, dt: f32) {
        self.state_timer += dt;
        self.update_stars(dt);
        self.update_particles(dt);
        self.update_blast_waves(dt);
        self.screen_shake = (self.screen_shake - dt * 18.0).max(0.0);
        self.shield_ring_timer = (self.shield_ring_timer - dt).max(0.0);
        self.rapid_fire_timer = (self.rapid_fire_timer - dt).max(0.0);
        self.player.cooldown = (self.player.cooldown - dt).max(0.0);
        self.player.hit_flash = (self.player.hit_flash - dt * 3.0).max(0.0);
        self.idle_aliens(0.0);
        if self.state_timer >= config::WAVE_CLEAR_TIME {
            self.wave += 1;
            self.setup_wave();
            self.screen = ScreenState::WaveIntro;
            self.state_timer = 0.0;
        }
    }

    fn update_game_over(&mut self, dt: f32) {
        self.state_timer += dt;
        self.update_stars(dt);
        self.update_particles(dt);
        self.update_blast_waves(dt);
        self.screen_shake = (self.screen_shake - dt * 18.0).max(0.0);
        self.shield_ring_timer = (self.shield_ring_timer - dt).max(0.0);
        self.rapid_fire_timer = (self.rapid_fire_timer - dt).max(0.0);
        self.player.cooldown = (self.player.cooldown - dt).max(0.0);
        self.player.hit_flash = (self.player.hit_flash - dt * 3.0).max(0.0);
        if is_key_pressed(KeyCode::Escape) {
            self.reset_for_new_run();
            self.screen = ScreenState::Title;
            self.state_timer = 0.0;
            return;
        }
        if self.state_timer >= config::GAME_OVER_DELAY
            && (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
        {
            self.reset_for_new_run();
            self.screen = ScreenState::WaveIntro;
            self.state_timer = 0.0;
        }
    }

    fn update_playing(&mut self, dt: f32) {
        self.state_timer += dt;
        self.update_stars(dt);
        self.update_particles(dt);
        self.update_blast_waves(dt);
        self.screen_shake = (self.screen_shake - dt * 18.0).max(0.0);
        self.shield_ring_timer = (self.shield_ring_timer - dt).max(0.0);
        self.rapid_fire_timer = (self.rapid_fire_timer - dt).max(0.0);
        self.player.cooldown = (self.player.cooldown - dt).max(0.0);
        self.player.hit_flash = (self.player.hit_flash - dt * 3.0).max(0.0);
        if is_key_pressed(KeyCode::Escape) {
            self.screen = ScreenState::Paused;
            self.state_timer = 0.0;
            return;
        }
        self.update_bomb_reward_window(dt);
        self.update_player(dt);
        self.update_shots(dt);
        self.update_aliens(dt);
        self.handle_collisions();
        self.enemy_fire_timer -= dt;
        if self.enemy_fire_timer <= 0.0 {
            self.spawn_enemy_shot();
            self.enemy_fire_timer = self.enemy_fire_interval();
        }

        if self.aliens.iter().all(|alien| !alien.alive) {
            self.play_wave_clear_sound();
            self.screen = ScreenState::WaveClear;
            self.state_timer = 0.0;
        } else if self.invaders_reached_player_zone() {
            self.trigger_game_over();
        }
    }

    fn update_paused(&mut self) {
        if is_key_pressed(KeyCode::Escape) {
            self.reset_for_new_run();
            self.screen = ScreenState::Title;
            self.state_timer = 0.0;
            return;
        }
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            self.screen = ScreenState::Playing;
            self.state_timer = 0.0;
        }
    }

    fn update_player(&mut self, dt: f32) {
        let mut move_dir = 0.0;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            move_dir -= 1.0;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            move_dir += 1.0;
        }
        self.player.x += move_dir * config::PLAYER_SPEED * dt;
        let half_w = config::PLAYER_WIDTH * 0.5;
        self.player.x = self
            .player
            .x
            .clamp(half_w + 20.0, config::WINDOW_WIDTH - half_w - 20.0);

        if is_key_pressed(KeyCode::Space)
            && self.player.cooldown <= 0.0
            && (self.rapid_fire_timer > 0.0
                || !self
                    .shots
                    .iter()
                    .any(|shot| shot.from_player && shot.kind == ShotKind::Bolt))
        {
            self.spawn_player_shot();
            self.player.cooldown = if self.rapid_fire_timer > 0.0 {
                config::PLAYER_RAPID_FIRE_COOLDOWN
            } else {
                config::PLAYER_COOLDOWN
            };
        }

        if is_key_pressed(KeyCode::Up)
            && self.player.cooldown <= 0.0
            && self.player.bombs > 0
            && !self
                .shots
                .iter()
                .any(|shot| shot.from_player && shot.kind == ShotKind::PlayerBomb)
        {
            self.spawn_player_bomb();
            self.player.cooldown = config::PLAYER_BOMB_COOLDOWN;
        }
    }

    fn update_shots(&mut self, dt: f32) {
        for shot in &mut self.shots {
            shot.pos += shot.vel * dt;
        }
        self.shots
            .retain(|shot| shot.pos.y > -40.0 && shot.pos.y < config::WINDOW_HEIGHT + 40.0);
    }

    fn update_aliens(&mut self, dt: f32) {
        for alien in &mut self.aliens {
            alien.fire_flash = (alien.fire_flash - dt * 4.5).max(0.0);
        }
        self.update_diving_aliens(dt);
        let alive = self.alive_aliens();
        let speed_scale =
            1.0 + (1.0 - alive as f32 / (config::ALIEN_ROWS * config::ALIEN_COLS) as f32) * 2.4;
        let march_speed = self.alien_dir
            * config::ALIEN_BASE_SPEED
            * speed_scale
            * (1.0 + self.wave as f32 * 0.08)
            * dt;
        self.formation_x += march_speed;
        self.march_progress += march_speed.abs();
        while self.march_progress >= config::ALIEN_MARCH_DISTANCE {
            self.march_progress -= config::ALIEN_MARCH_DISTANCE;
            self.toggle_alien_frames();
        }

        let (left, right, bottom) = self.alien_bounds();
        if left <= 30.0 || right >= config::WINDOW_WIDTH - 30.0 {
            self.alien_dir *= -1.0;
            self.formation_x = self.formation_x.clamp(
                self.formation_x + (30.0 - left).min(0.0),
                self.formation_x - (right - (config::WINDOW_WIDTH - 30.0)).max(0.0),
            );
            self.pass_count += 1;
            self.formation_y += config::ALIEN_STEP_DOWN
                + self.wave as f32 * 2.0
                + self.pass_count as f32 * config::ALIEN_STEP_DOWN_PASS_BONUS;
            self.toggle_alien_frames();
            self.screen_shake = self.screen_shake.max(7.0);
            self.spawn_radial_burst(
                vec2((left + right) * 0.5, bottom),
                12,
                config::ACCENT_A,
                140.0,
                2.0,
            );
        }

        self.dive_timer -= dt;
        if self.dive_timer <= 0.0 && !self.aliens.iter().any(|alien| alien.alive && alien.diving) {
            self.spawn_diving_alien();
            self.schedule_next_dive();
        }
    }

    fn handle_collisions(&mut self) {
        let mut remove_shots = vec![false; self.shots.len()];

        for (i, should_remove) in remove_shots.iter_mut().enumerate() {
            let shot = self.shots[i];
            let shot_rect = shot_rect(shot);
            let mut bunker_hit = false;

            for bunker in &mut self.bunkers {
                bunker_hit = match shot.kind {
                    ShotKind::Bolt => bunker.damage_at_rect(shot_rect, shot.from_player),
                    ShotKind::EnemyBomb | ShotKind::PlayerBomb => false,
                };
                if bunker_hit {
                    break;
                }
            }

            if bunker_hit {
                *should_remove = true;
                if shot.kind == ShotKind::EnemyBomb {
                    self.explode_bomb(shot.pos);
                } else {
                    self.spawn_impact(
                        shot_rect.center(),
                        if shot.from_player {
                            config::PLAYER_SHOT_COLOR
                        } else {
                            config::ENEMY_SHOT_COLOR
                        },
                    );
                }
            }

            if *should_remove {
                continue;
            }

            if shot.from_player {
                if shot.kind == ShotKind::PlayerBomb {
                    let hit_alien = self
                        .aliens
                        .iter()
                        .any(|alien| alien.alive && shot_rect.overlaps(&self.alien_rect(*alien)));
                    if hit_alien || shot.pos.y <= 110.0 {
                        *should_remove = true;
                        self.explode_player_bomb(shot.pos);
                    }
                } else {
                    let mut hit_index = None;
                    for (alien_index, alien) in self.aliens.iter().enumerate() {
                        if alien.alive && shot_rect.overlaps(&self.alien_rect(*alien)) {
                            hit_index = Some(alien_index);
                            break;
                        }
                    }

                    if let Some(alien_index) = hit_index {
                        let alien = self.aliens[alien_index];
                        self.aliens[alien_index].alive = false;
                        *should_remove = true;
                        self.register_alien_kill(alien, self.alien_rect(alien).center());
                    }
                }
            } else if shot.kind == ShotKind::EnemyBomb
                && shot.pos.y >= config::BUNKER_Y - config::BOMB_DETONATION_Y_OFFSET
            {
                *should_remove = true;
                self.explode_bomb(vec2(
                    shot.pos.x,
                    config::BUNKER_Y - config::BOMB_DETONATION_Y_OFFSET,
                ));
            } else if shot_rect.overlaps(&self.player_rect()) {
                *should_remove = true;
                self.damage_player();
            }
        }

        for i in 0..self.shots.len() {
            if remove_shots[i] {
                continue;
            }
            for j in (i + 1)..self.shots.len() {
                if self.shots[i].from_player == self.shots[j].from_player || remove_shots[j] {
                    continue;
                }
                if shot_rect(self.shots[i]).overlaps(&shot_rect(self.shots[j])) {
                    remove_shots[i] = true;
                    remove_shots[j] = true;
                    self.spawn_impact(
                        (self.shots[i].pos + self.shots[j].pos) * 0.5,
                        config::ACCENT_C,
                    );
                    break;
                }
            }
        }

        let mut idx = 0usize;
        self.shots.retain(|_| {
            let keep = !remove_shots[idx];
            idx += 1;
            keep
        });
    }

    fn draw_playfield(&self, offset: Vec2) {
        for bunker in &self.bunkers {
            bunker.draw(offset);
        }

        for alien in &self.aliens {
            if alien.alive {
                self.draw_alien(*alien, offset);
            }
        }

        for shot in &self.shots {
            draw_shot(*shot, offset);
        }

        self.draw_player(offset);

        for particle in &self.particles {
            let t = particle.life / particle.max_life;
            let glow = Color::new(
                particle.color.r,
                particle.color.g,
                particle.color.b,
                0.18 * t,
            );
            draw_circle(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                particle.size * (1.8 - t),
                glow,
            );
            draw_circle(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                particle.size * (0.55 + t * 0.45),
                Color::new(particle.color.r, particle.color.g, particle.color.b, t),
            );
        }

        for wave in &self.blast_waves {
            let alpha = (1.0 - wave.radius / config::BOMB_WAVE_MAX_RADIUS).clamp(0.0, 1.0);
            draw_circle_lines(
                wave.center.x + offset.x,
                wave.center.y + offset.y,
                wave.radius,
                6.0,
                Color::new(
                    config::ACCENT_C.r,
                    config::ACCENT_C.g,
                    config::ACCENT_C.b,
                    0.45 * alpha,
                ),
            );
            draw_circle_lines(
                wave.center.x + offset.x,
                wave.center.y + offset.y,
                wave.radius * 0.88,
                2.0,
                Color::new(
                    config::ACCENT_B.r,
                    config::ACCENT_B.g,
                    config::ACCENT_B.b,
                    0.35 * alpha,
                ),
            );
        }

        draw_line(
            36.0 + offset.x,
            config::PLAYER_ZONE_Y + offset.y,
            config::WINDOW_WIDTH - 36.0 + offset.x,
            config::PLAYER_ZONE_Y + offset.y,
            2.0,
            Color::from_rgba(255, 255, 255, 24),
        );
    }

    fn draw_hud(&self) {
        draw_arcade_panel(24.0, 18.0, 280.0, 56.0, config::ACCENT_A);
        draw_arcade_panel(320.0, 18.0, 320.0, 56.0, config::ACCENT_C);
        draw_arcade_panel(
            config::WINDOW_WIDTH - 514.0,
            18.0,
            150.0,
            56.0,
            config::ACCENT_B,
        );
        draw_arcade_panel(
            config::WINDOW_WIDTH - 356.0,
            18.0,
            180.0,
            56.0,
            config::ACCENT_C,
        );
        draw_arcade_panel(
            config::WINDOW_WIDTH - 168.0,
            18.0,
            144.0,
            56.0,
            config::ACCENT_A,
        );
        arcade_text(
            &format!("SCORE {:06}", self.score),
            38.0,
            56.0,
            28.0,
            config::ACCENT_A,
            false,
        );
        arcade_text(
            &format!("HIGH {:06}", self.high_score),
            480.0,
            56.0,
            28.0,
            config::ACCENT_C,
            true,
        );
        arcade_text(
            &format!("WAVE {}", self.wave),
            config::WINDOW_WIDTH - 498.0,
            56.0,
            24.0,
            config::ACCENT_B,
            false,
        );

        let lives_y = 56.0;
        arcade_text(
            "LIVES",
            config::WINDOW_WIDTH - 154.0,
            lives_y,
            18.0,
            config::ACCENT_A,
            false,
        );
        for i in 0..self.lives {
            let x = config::WINDOW_WIDTH - 145.0 + i as f32 * 24.0;
            draw_triangle(
                vec2(x, lives_y - 18.0),
                vec2(x - 11.0, lives_y + 2.0),
                vec2(x + 11.0, lives_y + 2.0),
                config::PLAYER_COLOR,
            );
            draw_triangle(
                vec2(x, lives_y - 14.0),
                vec2(x - 7.0, lives_y),
                vec2(x + 7.0, lives_y),
                mix_color(config::PLAYER_COLOR, WHITE, 0.3),
            );
        }

        let bombs_x = config::WINDOW_WIDTH - 342.0;
        arcade_text("BOMBS", bombs_x, 56.0, 18.0, config::ACCENT_C, false);
        for i in 0..self.player.bombs {
            let x = bombs_x + i as f32 * 26.0;
            let y = 44.0;
            draw_circle(x + 10.0, y, 11.0, Color::from_rgba(255, 170, 82, 80));
            draw_circle(x + 10.0, y, 7.5, config::PLAYER_BOMB_COLOR);
            draw_circle(x + 10.0, y, 3.5, WHITE);
            draw_line(
                x + 10.0,
                y - 8.0,
                x + 10.0,
                y - 18.0,
                3.0,
                Color::from_rgba(255, 228, 132, 255),
            );
        }

        if self.rapid_fire_timer > 0.0 {
            arcade_text_centered(
                &format!("RAPID FIRE {:.1}", self.rapid_fire_timer),
                config::WINDOW_WIDTH * 0.5,
                104.0,
                22.0,
                config::ACCENT_B,
            );
        }
    }

    fn draw_overlay(&self) {
        match self.screen {
            ScreenState::Title => {
                let fade = (self.state_timer / config::TITLE_FADE_TIME).min(1.0);
                draw_rectangle(
                    0.0,
                    0.0,
                    config::WINDOW_WIDTH,
                    config::WINDOW_HEIGHT,
                    Color::new(0.02, 0.03, 0.08, 0.55 * fade),
                );
                arcade_title(
                    "NEON INVADERS",
                    config::WINDOW_WIDTH * 0.5,
                    220.0,
                    96.0,
                    config::ACCENT_A,
                    true,
                );
                arcade_text_centered(
                    "Classic invasion. Modern arcade energy.",
                    config::WINDOW_WIDTH * 0.5,
                    300.0,
                    28.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Move: A / D or Left / Right    Shot: Space    Bomb: Up    Quit: Esc",
                    config::WINDOW_WIDTH * 0.5,
                    380.0,
                    22.0,
                    config::HUD_COLOR,
                );
                arcade_text_centered(
                    "Press Space to Start",
                    config::WINDOW_WIDTH * 0.5,
                    470.0 + self.state_timer.sin() * 8.0,
                    34.0,
                    config::ACCENT_C,
                );
            }
            ScreenState::WaveIntro => {
                let alpha = (1.0 - self.state_timer / config::WAVE_INTRO_TIME).clamp(0.0, 1.0);
                draw_rectangle(
                    0.0,
                    0.0,
                    config::WINDOW_WIDTH,
                    config::WINDOW_HEIGHT,
                    Color::new(0.02, 0.02, 0.06, 0.24 * alpha),
                );
                arcade_text_centered(
                    &format!("WAVE {}", self.wave),
                    config::WINDOW_WIDTH * 0.5,
                    360.0,
                    58.0,
                    Color::new(
                        config::ACCENT_A.r,
                        config::ACCENT_A.g,
                        config::ACCENT_A.b,
                        alpha,
                    ),
                );
            }
            ScreenState::Paused => {
                draw_rectangle(
                    0.0,
                    0.0,
                    config::WINDOW_WIDTH,
                    config::WINDOW_HEIGHT,
                    Color::from_rgba(4, 7, 20, 170),
                );
                arcade_title(
                    "PAUSED",
                    config::WINDOW_WIDTH * 0.5,
                    340.0,
                    84.0,
                    config::ACCENT_A,
                    true,
                );
                arcade_text_centered(
                    "Press Space to Resume",
                    config::WINDOW_WIDTH * 0.5,
                    420.0,
                    30.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Press Esc to Return to Title",
                    config::WINDOW_WIDTH * 0.5,
                    470.0,
                    28.0,
                    config::HUD_COLOR,
                );
            }
            ScreenState::WaveClear => {
                arcade_text_centered(
                    "WAVE CLEARED",
                    config::WINDOW_WIDTH * 0.5,
                    350.0,
                    54.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Incoming formation detected...",
                    config::WINDOW_WIDTH * 0.5,
                    410.0,
                    26.0,
                    config::HUD_COLOR,
                );
            }
            ScreenState::GameOver => {
                draw_rectangle(
                    0.0,
                    0.0,
                    config::WINDOW_WIDTH,
                    config::WINDOW_HEIGHT,
                    Color::from_rgba(8, 2, 16, 168),
                );
                arcade_title(
                    "GAME OVER",
                    config::WINDOW_WIDTH * 0.5,
                    320.0,
                    92.0,
                    config::ACCENT_B,
                    true,
                );
                arcade_text_centered(
                    &format!("Final Score {:06}", self.score),
                    config::WINDOW_WIDTH * 0.5,
                    405.0,
                    32.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Press Space to Restart or Esc for Title",
                    config::WINDOW_WIDTH * 0.5,
                    500.0,
                    28.0,
                    config::HUD_COLOR,
                );
            }
            ScreenState::Playing => {}
        }
    }

    fn draw_background(&self) {
        draw_rectangle(
            0.0,
            0.0,
            config::WINDOW_WIDTH,
            config::WINDOW_HEIGHT,
            config::BG_TOP,
        );
        for i in 0..10 {
            let y = i as f32 / 10.0 * config::WINDOW_HEIGHT;
            let t = i as f32 / 9.0;
            let color = Color::new(
                config::BG_TOP.r + (config::BG_MID.r - config::BG_TOP.r) * (t * 0.65),
                config::BG_TOP.g + (config::BG_MID.g - config::BG_TOP.g) * (t * 0.65),
                config::BG_TOP.b + (config::BG_BOTTOM.b - config::BG_TOP.b) * t,
                1.0,
            );
            draw_rectangle(
                0.0,
                y,
                config::WINDOW_WIDTH,
                config::WINDOW_HEIGHT / 10.0 + 2.0,
                color,
            );
        }

        for x in [260.0, 960.0, 1650.0] {
            draw_circle(x, 150.0, 110.0, Color::from_rgba(255, 55, 82, 9));
        }
        for x in [420.0, 1280.0] {
            draw_circle(x, 240.0, 160.0, Color::from_rgba(64, 122, 255, 11));
        }

        for (layer, stars) in self.stars.iter().enumerate() {
            let tint = match layer {
                0 => WHITE,
                1 => config::ACCENT_C,
                _ => WHITE,
            };
            for star in stars {
                let glow = Color::new(tint.r, tint.g, tint.b, star.alpha * 0.12);
                draw_circle(star.pos.x, star.pos.y, star.radius * 2.1, glow);
                draw_circle(
                    star.pos.x,
                    star.pos.y,
                    star.radius,
                    Color::new(tint.r, tint.g, tint.b, star.alpha),
                );
            }
        }

        draw_rectangle(
            0.0,
            config::WINDOW_HEIGHT - 210.0,
            config::WINDOW_WIDTH,
            210.0,
            Color::from_rgba(6, 8, 22, 72),
        );
    }

    fn draw_player(&self, offset: Vec2) {
        let mut rect = self.player_rect();
        rect.x += offset.x;
        rect.y += offset.y;
        let flash = self.player.hit_flash;
        let color = mix_color(config::PLAYER_COLOR, WHITE, flash * 0.65);
        let glow = Color::new(
            config::PLAYER_GLOW.r,
            config::PLAYER_GLOW.g,
            config::PLAYER_GLOW.b,
            0.22 + flash * 0.18,
        );
        let panel = mix_color(color, WHITE, 0.22);
        let underside = mix_color(color, BLACK, 0.42);
        let warm_panel = Color::from_rgba(255, 110, 82, 255);
        let cool_panel = Color::from_rgba(118, 232, 255, 255);
        draw_circle(rect.center().x, rect.center().y + 6.0, 52.0, glow);
        draw_triangle(
            vec2(rect.x + rect.w * 0.5, rect.y - 4.0),
            vec2(rect.x + 20.0, rect.y + rect.h + 12.0),
            vec2(rect.x + rect.w - 20.0, rect.y + rect.h + 12.0),
            underside,
        );
        draw_triangle(
            vec2(rect.x + rect.w * 0.5, rect.y - 16.0),
            vec2(rect.x + 12.0, rect.y + rect.h + 4.0),
            vec2(rect.x + rect.w - 12.0, rect.y + rect.h + 4.0),
            color,
        );
        draw_triangle(
            vec2(rect.x + rect.w * 0.5, rect.y - 8.0),
            vec2(rect.x + 20.0, rect.y + rect.h - 1.0),
            vec2(rect.x + rect.w - 20.0, rect.y + rect.h - 1.0),
            panel,
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 18.0,
            rect.y + 10.0,
            36.0,
            rect.h - 2.0,
            color,
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 18.0,
            rect.y + rect.h - 6.0,
            36.0,
            10.0,
            underside,
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 12.0,
            rect.y + 14.0,
            24.0,
            7.0,
            Color::new(panel.r, panel.g, panel.b, 0.9),
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 8.0,
            rect.y + 24.0,
            16.0,
            10.0,
            cool_panel,
        );
        draw_triangle(
            vec2(rect.x + 20.0, rect.y + rect.h + 2.0),
            vec2(rect.x + 34.0, rect.y + 10.0),
            vec2(rect.x + 42.0, rect.y + rect.h + 2.0),
            Color::from_rgba(255, 84, 74, 255),
        );
        draw_triangle(
            vec2(rect.x + rect.w - 20.0, rect.y + rect.h + 2.0),
            vec2(rect.x + rect.w - 34.0, rect.y + 10.0),
            vec2(rect.x + rect.w - 42.0, rect.y + rect.h + 2.0),
            Color::from_rgba(255, 84, 74, 255),
        );
        draw_triangle(
            vec2(rect.x + 24.0, rect.y + rect.h + 1.0),
            vec2(rect.x + 34.0, rect.y + 18.0),
            vec2(rect.x + 36.0, rect.y + rect.h + 1.0),
            warm_panel,
        );
        draw_triangle(
            vec2(rect.x + rect.w - 24.0, rect.y + rect.h + 1.0),
            vec2(rect.x + rect.w - 34.0, rect.y + 18.0),
            vec2(rect.x + rect.w - 36.0, rect.y + rect.h + 1.0),
            warm_panel,
        );
        draw_line(
            rect.x + 12.0,
            rect.y + rect.h + 2.0,
            rect.x + rect.w - 12.0,
            rect.y + rect.h + 2.0,
            2.0,
            Color::from_rgba(255, 255, 255, 55),
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 6.0,
            rect.y - 20.0,
            12.0,
            30.0,
            Color::from_rgba(255, 216, 108, 255),
        );
        draw_rectangle(rect.x + rect.w * 0.5 - 3.0, rect.y - 15.0, 6.0, 13.0, WHITE);
        let protected = self.player_protected_by_bunker();
        let shield_alpha = if protected {
            0.28 + self.shield_ring_timer.clamp(0.0, 0.45) * 0.6
        } else {
            (self.shield_ring_timer / 0.45).clamp(0.0, 1.0) * 0.55
        };
        if shield_alpha > 0.0 {
            let pulse = get_time() as f32 * 3.5;
            let radius_offset = if protected {
                pulse.sin() * 2.5
            } else {
                (1.0 - shield_alpha) * 10.0
            };
            draw_circle_lines(
                rect.center().x,
                rect.center().y - 6.0,
                58.0 + radius_offset.max(-2.0),
                4.0,
                Color::new(
                    config::ACCENT_A.r,
                    config::ACCENT_A.g,
                    config::ACCENT_A.b,
                    shield_alpha,
                ),
            );
            draw_circle_lines(
                rect.center().x,
                rect.center().y - 6.0,
                68.0 + radius_offset * 1.15 + 4.0,
                2.0,
                Color::new(
                    config::ACCENT_C.r,
                    config::ACCENT_C.g,
                    config::ACCENT_C.b,
                    shield_alpha * 0.8,
                ),
            );
        }
    }

    fn draw_alien(&self, alien: Alien, offset: Vec2) {
        if alien.diving {
            self.draw_diving_alien(alien, offset);
            return;
        }
        let rect = self.alien_rect(alien);
        let color = alien_color(alien.row);
        let center = rect.center() + offset;
        let fire_flash = alien.fire_flash.clamp(0.0, 1.0);
        let highlight = mix_color(color, WHITE, 0.42);
        let shadow = mix_color(color, BLACK, 0.28);
        let belly = mix_color(color, BLACK, 0.48);
        let canopy = mix_color(color, WHITE, 0.68);
        let sweep_phase =
            get_time() as f32 * 2.8 + self.march_step as f32 * 0.75 + alien.col as f32 * 0.18;
        let sweep = (sweep_phase.sin() * 0.5 + 0.5).powf(2.2);
        let flicker = if alien.frame { 1.0 } else { 0.0 };
        let glow = Color::new(
            color.r,
            color.g,
            color.b,
            0.16 + sweep * 0.08 + flicker * 0.03 + fire_flash * 0.18,
        );
        let canopy_sweep = mix_color(canopy, WHITE, 0.18 + sweep * 0.55 + fire_flash * 0.3);
        let wing_glint = Color::new(
            highlight.r,
            highlight.g,
            highlight.b,
            0.16 + sweep * 0.28 + flicker * 0.08 + fire_flash * 0.22,
        );
        let accent = match alien.row {
            0 => Color::from_rgba(255, 92, 82, 255),
            1 => Color::from_rgba(255, 231, 120, 255),
            2 => Color::from_rgba(110, 232, 255, 255),
            3 => Color::from_rgba(255, 140, 196, 255),
            _ => Color::from_rgba(255, 235, 145, 255),
        };
        let accent_alt = match alien.row {
            0 => Color::from_rgba(116, 226, 255, 255),
            1 => Color::from_rgba(255, 124, 196, 255),
            2 => Color::from_rgba(255, 166, 74, 255),
            3 => Color::from_rgba(138, 255, 178, 255),
            _ => Color::from_rgba(112, 212, 255, 255),
        };
        let glitter_phase =
            get_time() as f32 * 5.5 + alien.row as f32 * 0.8 + alien.col as f32 * 0.27;
        let glitter = (glitter_phase.sin() * 0.5 + 0.5).powf(2.6);
        let leg_sway = if alien.frame { 6.0 } else { -6.0 };
        draw_circle(center.x, center.y + 2.0, 44.0, glow);
        draw_circle(
            center.x - 8.0,
            center.y - 10.0,
            20.0,
            Color::new(
                highlight.r,
                highlight.g,
                highlight.b,
                0.10 + sweep * 0.08 + fire_flash * 0.1,
            ),
        );
        if fire_flash > 0.0 {
            let flash_color = Color::new(accent.r, accent.g, accent.b, 0.18 + fire_flash * 0.26);
            draw_circle(
                center.x,
                center.y + 18.0,
                18.0 + fire_flash * 8.0,
                flash_color,
            );
            draw_circle(
                center.x,
                center.y + 25.0,
                10.0 + fire_flash * 6.0,
                Color::new(1.0, 0.95, 0.8, 0.16 + fire_flash * 0.22),
            );
        }

        match alien.row {
            0 => {
                draw_ellipse(center.x, center.y + 5.0, 25.0, 13.0, 0.0, shadow);
                draw_ellipse(center.x, center.y + 1.0, 24.0, 16.0, 0.0, color);
                draw_ellipse(center.x, center.y + 8.0, 21.0, 6.0, 0.0, belly);
                draw_ellipse(center.x, center.y + 4.0, 18.0, 8.0, 0.0, accent_alt);
                draw_ellipse(center.x, center.y - 4.0, 16.0, 7.0, 0.0, canopy_sweep);
                draw_ellipse(
                    center.x + sweep * 7.0 - 3.5,
                    center.y - 6.0,
                    4.0,
                    2.0,
                    0.0,
                    wing_glint,
                );
                draw_triangle(
                    vec2(center.x - 18.0, center.y - 3.0),
                    vec2(center.x - 35.0, center.y + 10.0),
                    vec2(center.x - 18.0, center.y + 8.0),
                    color,
                );
                draw_triangle(
                    vec2(center.x - 14.0, center.y - 2.0),
                    vec2(center.x - 28.0, center.y + 7.0),
                    vec2(center.x - 15.0, center.y + 5.0),
                    accent,
                );
                draw_triangle(
                    vec2(center.x - 9.0, center.y + 0.0),
                    vec2(center.x - 22.0, center.y + 9.0),
                    vec2(center.x - 9.0, center.y + 7.0),
                    accent_alt,
                );
                draw_triangle(
                    vec2(center.x + 18.0, center.y - 3.0),
                    vec2(center.x + 35.0, center.y + 10.0),
                    vec2(center.x + 18.0, center.y + 8.0),
                    color,
                );
                draw_triangle(
                    vec2(center.x + 14.0, center.y - 2.0),
                    vec2(center.x + 28.0, center.y + 7.0),
                    vec2(center.x + 15.0, center.y + 5.0),
                    accent,
                );
                draw_triangle(
                    vec2(center.x + 9.0, center.y + 0.0),
                    vec2(center.x + 22.0, center.y + 9.0),
                    vec2(center.x + 9.0, center.y + 7.0),
                    accent_alt,
                );
                draw_rectangle(center.x - 8.0, center.y - 1.0, 16.0, 5.0, accent);
                draw_line(
                    center.x - 19.0,
                    center.y - 2.0,
                    center.x - 31.0,
                    center.y + 8.0,
                    2.0,
                    canopy_sweep,
                );
                draw_line(
                    center.x + 19.0,
                    center.y - 2.0,
                    center.x + 31.0,
                    center.y + 8.0,
                    2.0,
                    canopy_sweep,
                );
                draw_line(
                    center.x - 15.0,
                    center.y + 10.0,
                    center.x - 15.0,
                    center.y + 22.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x + 15.0,
                    center.y + 10.0,
                    center.x + 15.0,
                    center.y + 22.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x - 15.0,
                    center.y + 22.0,
                    center.x - 15.0 + leg_sway,
                    center.y + 30.0,
                    4.0,
                    accent_alt,
                );
                draw_line(
                    center.x + 15.0,
                    center.y + 22.0,
                    center.x + 15.0 - leg_sway,
                    center.y + 30.0,
                    4.0,
                    accent_alt,
                );
                draw_circle(
                    center.x - 14.0,
                    center.y - 8.0,
                    2.0 + glitter * 2.4,
                    Color::new(1.0, 1.0, 1.0, 0.45 + glitter * 0.45),
                );
            }
            1 => {
                draw_triangle(
                    vec2(center.x, center.y - 19.0),
                    vec2(center.x - 22.0, center.y + 6.0),
                    vec2(center.x + 22.0, center.y + 6.0),
                    shadow,
                );
                draw_triangle(
                    vec2(center.x, center.y - 22.0),
                    vec2(center.x - 26.0, center.y + 8.0),
                    vec2(center.x + 26.0, center.y + 8.0),
                    color,
                );
                draw_triangle(
                    vec2(center.x, center.y - 16.0),
                    vec2(center.x - 17.0, center.y + 2.0),
                    vec2(center.x + 17.0, center.y + 2.0),
                    canopy_sweep,
                );
                draw_triangle(
                    vec2(center.x, center.y + 6.0),
                    vec2(center.x - 18.0, center.y + 12.0),
                    vec2(center.x + 18.0, center.y + 12.0),
                    belly,
                );
                draw_triangle(
                    vec2(center.x + sweep * 10.0 - 5.0, center.y - 12.0),
                    vec2(center.x + sweep * 10.0 - 11.0, center.y - 1.0),
                    vec2(center.x + sweep * 10.0 + 1.0, center.y - 1.0),
                    wing_glint,
                );
                draw_triangle(
                    vec2(center.x, center.y - 7.0),
                    vec2(center.x - 10.0, center.y + 7.0),
                    vec2(center.x + 10.0, center.y + 7.0),
                    accent_alt,
                );
                draw_circle(center.x - 14.0, center.y + 2.0, 4.0, accent);
                draw_circle(center.x + 14.0, center.y + 2.0, 4.0, accent);
                draw_rectangle(center.x - 19.0, center.y + 5.0, 38.0, 4.0, accent_alt);
                draw_line(
                    center.x - 20.0,
                    center.y - 4.0,
                    center.x - 8.0,
                    center.y + 3.0,
                    2.0,
                    canopy_sweep,
                );
                draw_line(
                    center.x + 20.0,
                    center.y - 4.0,
                    center.x + 8.0,
                    center.y + 3.0,
                    2.0,
                    canopy_sweep,
                );
                draw_circle(center.x - 9.0, center.y - 2.0, 4.0, BLACK);
                draw_circle(center.x + 9.0, center.y - 2.0, 4.0, BLACK);
                draw_line(
                    center.x - 12.0,
                    center.y + 8.0,
                    center.x - 12.0,
                    center.y + 22.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x + 12.0,
                    center.y + 8.0,
                    center.x + 12.0,
                    center.y + 22.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x - 12.0,
                    center.y + 22.0,
                    center.x - 12.0 + leg_sway,
                    center.y + 31.0,
                    4.0,
                    accent,
                );
                draw_line(
                    center.x + 12.0,
                    center.y + 22.0,
                    center.x + 12.0 - leg_sway,
                    center.y + 31.0,
                    4.0,
                    accent,
                );
                draw_circle(
                    center.x + 12.0,
                    center.y + 4.0,
                    1.5 + glitter * 2.0,
                    Color::new(1.0, 1.0, 1.0, 0.38 + glitter * 0.5),
                );
            }
            2 => {
                draw_ellipse(center.x, center.y + 2.0, 19.0, 17.0, 0.0, shadow);
                draw_ellipse(center.x, center.y - 2.0, 18.0, 18.0, 0.0, color);
                draw_ellipse(center.x, center.y + 7.0, 15.0, 7.0, 0.0, belly);
                draw_ellipse(center.x, center.y + 1.0, 12.0, 11.0, 0.0, accent_alt);
                draw_rectangle(center.x - 14.0, center.y + 8.0, 28.0, 8.0, shadow);
                draw_triangle(
                    vec2(center.x, center.y - 26.0),
                    vec2(center.x - 14.0, center.y - 8.0),
                    vec2(center.x + 14.0, center.y - 8.0),
                    color,
                );
                draw_ellipse(center.x, center.y - 8.0, 11.0, 7.0, 0.0, canopy_sweep);
                draw_rectangle(center.x - 12.0, center.y + 3.0, 24.0, 6.0, accent);
                draw_rectangle(center.x - 6.0, center.y - 12.0, 12.0, 7.0, accent_alt);
                draw_rectangle(center.x - 10.0, center.y - 1.0, 20.0, 5.0, canopy_sweep);
                draw_rectangle(
                    center.x + sweep * 8.0 - 10.0,
                    center.y - 10.0,
                    5.0,
                    16.0,
                    wing_glint,
                );
                draw_circle(center.x - 8.0, center.y - 2.0, 3.0, BLACK);
                draw_circle(center.x + 8.0, center.y - 2.0, 3.0, BLACK);
                draw_line(
                    center.x - 10.0,
                    center.y + 16.0,
                    center.x - 10.0,
                    center.y + 28.0,
                    4.0,
                    color,
                );
                draw_line(
                    center.x + 10.0,
                    center.y + 16.0,
                    center.x + 10.0,
                    center.y + 28.0,
                    4.0,
                    color,
                );
                draw_line(
                    center.x - 10.0,
                    center.y + 28.0,
                    center.x - 10.0 + leg_sway,
                    center.y + 34.0,
                    3.5,
                    accent,
                );
                draw_line(
                    center.x + 10.0,
                    center.y + 28.0,
                    center.x + 10.0 - leg_sway,
                    center.y + 34.0,
                    3.5,
                    accent,
                );
                draw_circle(
                    center.x,
                    center.y + 2.0,
                    2.0 + glitter * 1.8,
                    Color::new(1.0, 1.0, 1.0, 0.35 + glitter * 0.5),
                );
            }
            3 => {
                draw_triangle(
                    vec2(center.x, center.y - 19.0),
                    vec2(center.x - 29.0, center.y + 9.0),
                    vec2(center.x + 29.0, center.y + 9.0),
                    shadow,
                );
                draw_triangle(
                    vec2(center.x, center.y - 24.0),
                    vec2(center.x - 34.0, center.y + 10.0),
                    vec2(center.x + 34.0, center.y + 10.0),
                    color,
                );
                draw_triangle(
                    vec2(center.x, center.y - 16.0),
                    vec2(center.x - 19.0, center.y + 1.0),
                    vec2(center.x + 19.0, center.y + 1.0),
                    canopy_sweep,
                );
                draw_triangle(
                    vec2(center.x, center.y + 5.0),
                    vec2(center.x - 24.0, center.y + 11.0),
                    vec2(center.x + 24.0, center.y + 11.0),
                    belly,
                );
                draw_triangle(
                    vec2(center.x, center.y - 8.0),
                    vec2(center.x - 14.0, center.y + 5.0),
                    vec2(center.x + 14.0, center.y + 5.0),
                    accent_alt,
                );
                draw_line(
                    center.x - 10.0,
                    center.y - 5.0,
                    center.x,
                    center.y + 8.0,
                    4.0,
                    accent,
                );
                draw_line(
                    center.x + 10.0,
                    center.y - 5.0,
                    center.x,
                    center.y + 8.0,
                    4.0,
                    accent,
                );
                draw_rectangle(center.x - 18.0, center.y + 6.0, 36.0, 3.0, accent);
                draw_line(
                    center.x - 22.0,
                    center.y - 7.0,
                    center.x - 8.0,
                    center.y + 2.0,
                    2.0,
                    canopy_sweep,
                );
                draw_line(
                    center.x + 22.0,
                    center.y - 7.0,
                    center.x + 8.0,
                    center.y + 2.0,
                    2.0,
                    canopy_sweep,
                );
                draw_triangle(
                    vec2(center.x + sweep * 14.0 - 11.0, center.y - 15.0),
                    vec2(center.x + sweep * 14.0 - 18.0, center.y - 1.0),
                    vec2(center.x + sweep * 14.0 - 4.0, center.y - 1.0),
                    wing_glint,
                );
                draw_line(
                    center.x - 12.0,
                    center.y + 10.0,
                    center.x - 12.0,
                    center.y + 24.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x + 12.0,
                    center.y + 10.0,
                    center.x + 12.0,
                    center.y + 24.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x - 12.0,
                    center.y + 24.0,
                    center.x - 12.0 + leg_sway,
                    center.y + 33.0,
                    4.0,
                    accent_alt,
                );
                draw_line(
                    center.x + 12.0,
                    center.y + 24.0,
                    center.x + 12.0 - leg_sway,
                    center.y + 33.0,
                    4.0,
                    accent_alt,
                );
                draw_circle(
                    center.x,
                    center.y + 3.0,
                    7.0,
                    Color::from_rgba(255, 255, 255, 40),
                );
                draw_circle(
                    center.x - 6.0,
                    center.y - 3.0,
                    1.6 + glitter * 2.0,
                    Color::new(1.0, 1.0, 1.0, 0.34 + glitter * 0.48),
                );
            }
            _ => {
                draw_ellipse(center.x, center.y + 4.0, 16.0, 17.0, 0.0, shadow);
                draw_ellipse(center.x, center.y + 1.0, 16.0, 20.0, 0.0, color);
                draw_ellipse(center.x, center.y + 9.0, 12.0, 7.0, 0.0, belly);
                draw_ellipse(center.x, center.y + 4.0, 9.0, 10.0, 0.0, accent_alt);
                draw_rectangle(center.x - 11.0, center.y + 10.0, 22.0, 7.0, shadow);
                draw_triangle(
                    vec2(center.x, center.y - 25.0),
                    vec2(center.x - 14.0, center.y - 5.0),
                    vec2(center.x + 14.0, center.y - 5.0),
                    color,
                );
                draw_triangle(
                    vec2(center.x, center.y - 16.0),
                    vec2(center.x - 12.0, center.y + 1.0),
                    vec2(center.x + 12.0, center.y + 1.0),
                    canopy_sweep,
                );
                draw_triangle(
                    vec2(center.x, center.y - 12.0),
                    vec2(center.x - 10.0, center.y - 1.0),
                    vec2(center.x + 10.0, center.y - 1.0),
                    accent,
                );
                draw_rectangle(center.x - 13.0, center.y + 8.0, 26.0, 4.0, accent_alt);
                draw_line(
                    center.x,
                    center.y - 16.0,
                    center.x,
                    center.y - 2.0,
                    2.0,
                    canopy_sweep,
                );
                draw_triangle(
                    vec2(center.x + sweep * 8.0 - 4.0, center.y - 14.0),
                    vec2(center.x + sweep * 8.0 - 9.0, center.y - 2.0),
                    vec2(center.x + sweep * 8.0 + 1.0, center.y - 2.0),
                    wing_glint,
                );
                draw_circle(center.x - 4.0, center.y + 2.0, 3.0, BLACK);
                draw_circle(center.x + 4.0, center.y + 2.0, 3.0, BLACK);
                draw_line(
                    center.x - 8.0,
                    center.y + 12.0,
                    center.x - 8.0,
                    center.y + 24.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x + 8.0,
                    center.y + 12.0,
                    center.x + 8.0,
                    center.y + 24.0,
                    5.0,
                    color,
                );
                draw_line(
                    center.x - 8.0,
                    center.y + 24.0,
                    center.x - 8.0 + leg_sway,
                    center.y + 32.0,
                    4.0,
                    accent,
                );
                draw_line(
                    center.x + 8.0,
                    center.y + 24.0,
                    center.x + 8.0 - leg_sway,
                    center.y + 32.0,
                    4.0,
                    accent,
                );
                draw_circle(
                    center.x + 6.0,
                    center.y - 6.0,
                    1.8 + glitter * 2.1,
                    Color::new(1.0, 1.0, 1.0, 0.36 + glitter * 0.46),
                );
            }
        }
    }

    fn draw_diving_alien(&self, alien: Alien, offset: Vec2) {
        let center = alien.dive_pos + offset;
        let color = alien_color(alien.row);
        let accent = mix_color(color, config::ACCENT_B, 0.45);
        let canopy = mix_color(color, WHITE, 0.65);
        let angle = alien.dive_angle;
        let glow = Color::new(color.r, color.g, color.b, 0.24 + alien.fire_flash * 0.16);
        draw_circle(center.x, center.y, 42.0, glow);

        let body_top = rotated_point(vec2(0.0, -24.0), angle) + center;
        let body_left = rotated_point(vec2(-19.0, 8.0), angle) + center;
        let body_right = rotated_point(vec2(19.0, 8.0), angle) + center;
        draw_triangle(body_top, body_left, body_right, color);

        let belly_top = rotated_point(vec2(0.0, -8.0), angle) + center;
        let belly_left = rotated_point(vec2(-14.0, 11.0), angle) + center;
        let belly_right = rotated_point(vec2(14.0, 11.0), angle) + center;
        draw_triangle(belly_top, belly_left, belly_right, accent);

        let canopy_center = rotated_point(vec2(0.0, -10.0), angle) + center;
        draw_circle(canopy_center.x, canopy_center.y, 8.0, canopy);

        let wing_l_a = rotated_point(vec2(-10.0, -4.0), angle) + center;
        let wing_l_b = rotated_point(vec2(-31.0, 2.0), angle) + center;
        let wing_l_c = rotated_point(vec2(-16.0, 16.0), angle) + center;
        draw_triangle(wing_l_a, wing_l_b, wing_l_c, accent);
        let wing_r_a = rotated_point(vec2(10.0, -4.0), angle) + center;
        let wing_r_b = rotated_point(vec2(31.0, 2.0), angle) + center;
        let wing_r_c = rotated_point(vec2(16.0, 16.0), angle) + center;
        draw_triangle(wing_r_a, wing_r_b, wing_r_c, accent);

        let leg1_a = rotated_point(vec2(-10.0, 10.0), angle) + center;
        let leg1_b = rotated_point(vec2(-10.0, 24.0), angle) + center;
        let leg1_c = rotated_point(vec2(-16.0, 33.0), angle) + center;
        draw_line(leg1_a.x, leg1_a.y, leg1_b.x, leg1_b.y, 4.0, color);
        draw_line(leg1_b.x, leg1_b.y, leg1_c.x, leg1_c.y, 3.0, canopy);
        let leg2_a = rotated_point(vec2(10.0, 10.0), angle) + center;
        let leg2_b = rotated_point(vec2(10.0, 24.0), angle) + center;
        let leg2_c = rotated_point(vec2(16.0, 33.0), angle) + center;
        draw_line(leg2_a.x, leg2_a.y, leg2_b.x, leg2_b.y, 4.0, color);
        draw_line(leg2_b.x, leg2_b.y, leg2_c.x, leg2_c.y, 3.0, canopy);
    }

    fn update_particles(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.pos += particle.vel * dt;
            particle.vel *= 0.98;
        }
        self.particles.retain(|particle| particle.life > 0.0);
    }

    fn update_blast_waves(&mut self, dt: f32) {
        let player_rect = self.player_rect();
        let player_center = player_rect.center();
        let mut pending_hits = Vec::new();

        for wave in &mut self.blast_waves {
            wave.radius += config::BOMB_WAVE_SPEED * dt;
        }

        for (index, wave) in self.blast_waves.iter().enumerate() {
            if wave.hit_player || wave.radius < wave.center.distance(player_center) {
                continue;
            }
            pending_hits.push((index, wave.center));
        }

        let mut hits_to_apply = 0u32;
        for (index, center) in pending_hits {
            self.blast_waves[index].hit_player = true;
            if self.player_shielded_from_bomb(center, player_rect) {
                self.shield_ring_timer = 0.45;
            } else {
                hits_to_apply += 1;
            }
        }

        self.blast_waves
            .retain(|wave| wave.radius < config::BOMB_WAVE_MAX_RADIUS);

        if hits_to_apply > 0 {
            self.damage_player();
        }
    }

    fn update_bomb_reward_window(&mut self, dt: f32) {
        self.kill_window_timer -= dt;
        if self.kill_window_timer > 0.0 {
            return;
        }

        let awarded = self.kill_window_kills / config::BOMB_REWARD_KILLS;
        if awarded > 0 {
            self.player.bombs += awarded;
            self.spawn_radial_burst(
                vec2(
                    self.player.x,
                    config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - 8.0,
                ),
                14,
                config::PLAYER_BOMB_COLOR,
                150.0,
                0.8,
            );
        }
        self.kill_window_kills = 0;
        self.kill_window_timer += config::BOMB_REWARD_WINDOW;
    }

    fn update_stars(&mut self, dt: f32) {
        for stars in &mut self.stars {
            for star in stars {
                star.pos.y += star.speed * dt;
                if star.pos.y > config::WINDOW_HEIGHT + 10.0 {
                    star.pos.y = -10.0;
                    star.pos.x = rand::gen_range(0.0, config::WINDOW_WIDTH);
                }
            }
        }
    }

    fn idle_aliens(&mut self, dt: f32) {
        for alien in &mut self.aliens {
            alien.fire_flash = (alien.fire_flash - dt * 4.5).max(0.0);
        }
        self.march_progress += dt * 22.0;
        while self.march_progress >= config::ALIEN_MARCH_DISTANCE {
            self.march_progress -= config::ALIEN_MARCH_DISTANCE;
            self.toggle_alien_frames();
        }
    }

    fn reset_for_new_run(&mut self) {
        self.score = 0;
        self.lives = 3;
        self.wave = 1;
        self.player.bombs = 0;
        self.kill_window_timer = config::BOMB_REWARD_WINDOW;
        self.kill_window_kills = 0;
        self.pending_restart = false;
        self.stars = build_stars();
        self.setup_wave();
    }

    fn setup_wave(&mut self) {
        self.player = Player {
            x: config::WINDOW_WIDTH * 0.5,
            cooldown: 0.0,
            hit_flash: 0.0,
            bombs: self.player.bombs,
        };
        self.shots.clear();
        self.particles.clear();
        self.blast_waves.clear();
        self.shield_ring_timer = 0.0;
        self.rapid_fire_timer = 0.0;
        self.bunkers = build_bunkers();
        self.aliens = build_aliens();
        self.formation_x = config::ALIEN_START_X;
        self.formation_y = config::ALIEN_START_Y + (self.wave as f32 - 1.0) * 12.0;
        self.alien_dir = 1.0;
        self.march_progress = 0.0;
        self.march_step = 0;
        self.pass_count = 0;
        self.dive_timer = config::ALIEN_DIVE_BASE_INTERVAL;
        self.kill_window_timer = config::BOMB_REWARD_WINDOW;
        self.kill_window_kills = 0;
        self.enemy_fire_timer =
            (config::ENEMY_FIRE_BASE_INTERVAL - self.wave as f32 * 0.07).max(0.5);
    }

    fn schedule_next_dive(&mut self) {
        let base = (config::ALIEN_DIVE_BASE_INTERVAL - self.wave as f32 * 0.35)
            .max(config::ALIEN_DIVE_MIN_INTERVAL);
        self.dive_timer = rand::gen_range(base * 0.7, base * 1.25);
    }

    fn spawn_diving_alien(&mut self) {
        let mut candidates = Vec::new();
        for (index, alien) in self.aliens.iter().enumerate() {
            if alien.alive && !alien.diving && alien.row >= 1 {
                candidates.push(index);
            }
        }
        if candidates.is_empty() {
            return;
        }
        let index = candidates[rand::gen_range(0, candidates.len())];
        let rect = self.alien_rect(self.aliens[index]);
        let center = rect.center();
        let target_x = self.player.x + rand::gen_range(-80.0, 80.0);
        self.aliens[index].diving = true;
        self.aliens[index].dive_pos = center;
        self.aliens[index].dive_vel = vec2(
            (target_x - center.x) * 0.7,
            config::ALIEN_DIVE_SPEED + self.wave as f32 * 20.0,
        );
        self.aliens[index].dive_angle = 0.0;
        self.aliens[index].fire_flash = 1.0;
        self.screen_shake = self.screen_shake.max(5.0);
        self.spawn_radial_burst(center, 10, alien_color(self.aliens[index].row), 120.0, 0.45);
    }

    fn update_diving_aliens(&mut self, dt: f32) {
        let player_rect = self.player_rect();
        let player_center = player_rect.center();
        let mut player_hit = false;
        for alien in &mut self.aliens {
            if !alien.alive || !alien.diving {
                continue;
            }
            let steer = (player_center.x - alien.dive_pos.x) * config::ALIEN_DIVE_TURN_RATE;
            alien.dive_vel.x += steer * dt;
            alien.dive_pos += alien.dive_vel * dt;
            alien.dive_angle += config::ALIEN_DIVE_SPIN_SPEED * dt;
            alien.fire_flash = alien.fire_flash.max(0.25);
            let dive_rect = Rect::new(
                alien.dive_pos.x - config::ALIEN_SIZE.x * 0.6,
                alien.dive_pos.y - config::ALIEN_SIZE.y * 0.75,
                config::ALIEN_SIZE.x * 1.2,
                config::ALIEN_SIZE.y * 1.5,
            );
            if dive_rect.overlaps(&player_rect) || alien.dive_pos.y >= config::PLAYER_ZONE_Y - 12.0
            {
                player_hit = true;
                break;
            }
        }

        if player_hit {
            self.trigger_game_over();
            return;
        }

        for alien in &mut self.aliens {
            if alien.alive
                && alien.diving
                && (alien.dive_pos.y > config::WINDOW_HEIGHT + 140.0
                    || alien.dive_pos.x < -120.0
                    || alien.dive_pos.x > config::WINDOW_WIDTH + 120.0)
            {
                alien.diving = false;
                alien.dive_pos = Vec2::ZERO;
                alien.dive_vel = Vec2::ZERO;
                alien.dive_angle = 0.0;
            }
        }
    }

    fn spawn_player_shot(&mut self) {
        self.shots.push(Shot {
            pos: vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - 30.0,
            ),
            vel: vec2(0.0, -config::PLAYER_SHOT_SPEED),
            size: vec2(config::SHOT_WIDTH, config::PLAYER_SHOT_HEIGHT),
            from_player: true,
            kind: ShotKind::Bolt,
        });
        self.play_player_shoot_sound();
        self.spawn_radial_burst(
            vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - 34.0,
            ),
            8,
            config::PLAYER_SHOT_COLOR,
            90.0,
            0.45,
        );
    }

    fn spawn_player_bomb(&mut self) {
        self.player.bombs -= 1;
        self.shots.push(Shot {
            pos: vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - 34.0,
            ),
            vel: vec2(0.0, -config::PLAYER_BOMB_SPEED),
            size: vec2(config::PLAYER_BOMB_WIDTH, config::PLAYER_BOMB_HEIGHT),
            from_player: true,
            kind: ShotKind::PlayerBomb,
        });
        self.spawn_radial_burst(
            vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - 38.0,
            ),
            10,
            config::PLAYER_BOMB_COLOR,
            110.0,
            0.55,
        );
    }

    fn spawn_enemy_shot(&mut self) {
        let mut columns = Vec::new();
        for col in 0..config::ALIEN_COLS {
            let mut candidate = None;
            for row in (0..config::ALIEN_ROWS).rev() {
                if let Some(alien) = self.aliens.iter().find(|alien| {
                    alien.col == col && alien.row == row && alien.alive && !alien.diving
                }) {
                    candidate = Some(*alien);
                    break;
                }
            }
            if let Some(alien) = candidate {
                columns.push(alien);
            }
        }

        if columns.is_empty() {
            return;
        }

        columns.sort_by(|a, b| {
            let ax = self.alien_rect(*a).center().x;
            let bx = self.alien_rect(*b).center().x;
            let ad = (ax - self.player.x).abs();
            let bd = (bx - self.player.x).abs();
            ad.partial_cmp(&bd).unwrap_or(std::cmp::Ordering::Equal)
        });
        let pick_pool = columns.len().min(3);
        let shooter = columns[rand::gen_range(0, pick_pool)];
        let rect = self.alien_rect(shooter);
        let bomb_allowed = !self
            .shots
            .iter()
            .any(|shot| !shot.from_player && shot.kind == ShotKind::EnemyBomb);
        let fire_bomb = bomb_allowed
            && rand::gen_range(0.0, 1.0)
                < (config::BOMB_FIRE_CHANCE + self.wave as f32 * 0.015).min(0.32);
        let target_x = self.player.x;
        let dx = target_x - rect.center().x;
        let speed = if fire_bomb {
            config::BOMB_SPEED + self.wave as f32 * 9.0
        } else {
            config::ENEMY_SHOT_SPEED + self.wave as f32 * 18.0
        };
        let time_to_target =
            ((config::PLAYER_ZONE_Y - rect.center().y).max(80.0) / speed).max(0.15);
        let aim_x = (dx / time_to_target * config::ENEMY_AIM_BIAS).clamp(
            -config::ENEMY_SHOT_MAX_X_SPEED,
            config::ENEMY_SHOT_MAX_X_SPEED,
        );
        if let Some(alien) = self
            .aliens
            .iter_mut()
            .find(|alien| alien.row == shooter.row && alien.col == shooter.col && alien.alive)
        {
            alien.fire_flash = 1.0;
        }
        self.shots.push(Shot {
            pos: vec2(rect.center().x, rect.y + rect.h * 0.75),
            vel: vec2(aim_x, speed),
            size: if fire_bomb {
                vec2(config::BOMB_WIDTH, config::BOMB_HEIGHT)
            } else {
                vec2(config::SHOT_WIDTH, config::ENEMY_SHOT_HEIGHT)
            },
            from_player: false,
            kind: if fire_bomb {
                ShotKind::EnemyBomb
            } else {
                ShotKind::Bolt
            },
        });
        self.play_enemy_shoot_sound();
        self.spawn_radial_burst(rect.center(), 6, alien_color(shooter.row), 75.0, 0.35);
    }

    fn spawn_enemy_explosion(&mut self, center: Vec2, color: Color) {
        self.play_enemy_explosion_sound();
        self.spawn_radial_burst(center, 18, color, 220.0, 0.9);
        self.spawn_radial_burst(center, 10, config::ACCENT_C, 140.0, 0.6);
    }

    fn spawn_impact(&mut self, center: Vec2, color: Color) {
        self.spawn_radial_burst(center, 8, color, 120.0, 0.35);
    }

    fn explode_player_bomb(&mut self, center: Vec2) {
        self.screen_shake = self.screen_shake.max(12.0);
        self.spawn_radial_burst(center, 24, config::PLAYER_BOMB_COLOR, 240.0, 0.9);
        self.spawn_radial_burst(center, 16, config::ACCENT_A, 180.0, 0.75);

        let mut killed = Vec::new();
        for (index, alien) in self.aliens.iter().enumerate() {
            if alien.alive
                && self.alien_rect(*alien).center().distance(center) <= config::PLAYER_BOMB_RADIUS
            {
                killed.push(index);
            }
        }

        for index in killed {
            let alien = self.aliens[index];
            if self.aliens[index].alive {
                self.aliens[index].alive = false;
                self.register_alien_kill(alien, self.alien_rect(alien).center());
            }
        }

        for shot in &mut self.shots {
            if !shot.from_player && shot.pos.distance(center) <= config::PLAYER_BOMB_RADIUS {
                shot.pos.y = -999.0;
            }
        }
    }

    fn spawn_radial_burst(
        &mut self,
        center: Vec2,
        count: usize,
        color: Color,
        speed: f32,
        life: f32,
    ) {
        for _ in 0..count {
            if self.particles.len() >= config::PARTICLE_CAP {
                break;
            }
            let angle = rand::gen_range(0.0, std::f32::consts::TAU);
            let particle_speed = rand::gen_range(speed * 0.35, speed);
            self.particles.push(Particle {
                pos: center,
                vel: vec2(angle.cos(), angle.sin()) * particle_speed,
                life: rand::gen_range(life * 0.6, life),
                max_life: life,
                size: rand::gen_range(2.0, 5.8),
                color,
            });
        }
    }

    fn damage_player(&mut self) {
        self.player.hit_flash = 1.0;
        self.screen_shake = self.screen_shake.max(14.0);
        self.play_player_hit_sound();
        self.spawn_radial_burst(
            vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET,
            ),
            20,
            config::ACCENT_B,
            230.0,
            0.95,
        );

        if self.lives > 0 {
            self.lives -= 1;
        }
        if self.lives == 0 {
            self.trigger_game_over();
        }
    }

    fn trigger_game_over(&mut self) {
        self.play_game_over_sound();
        self.screen = ScreenState::GameOver;
        self.state_timer = 0.0;
    }

    fn play_player_shoot_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_player_shoot();
        }
    }

    fn play_enemy_shoot_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_enemy_shoot();
        }
    }

    fn play_enemy_explosion_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_enemy_explosion();
        }
    }

    fn play_player_hit_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_player_hit();
        }
    }

    fn play_wave_clear_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_wave_clear();
        }
    }

    fn play_game_over_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_game_over();
        }
    }

    fn register_alien_kill(&mut self, alien: Alien, center: Vec2) {
        self.score += alien_score(alien.row);
        self.kill_window_kills += 1;
        self.screen_shake = self.screen_shake.max(10.0);
        self.spawn_enemy_explosion(center, alien_color(alien.row));
        if !self
            .aliens
            .iter()
            .any(|other| other.alive && other.row == alien.row)
        {
            self.rapid_fire_timer = config::ROW_CLEAR_RAPID_FIRE_TIME;
            self.spawn_radial_burst(center, 20, config::ACCENT_B, 180.0, 0.7);
        }
    }

    fn toggle_alien_frames(&mut self) {
        self.march_step = self.march_step.wrapping_add(1);
        self.play_march_sound();
        for alien in &mut self.aliens {
            alien.frame = !alien.frame;
        }
    }

    fn play_march_sound(&self) {
        if let Some(sounds) = &self.sounds {
            sounds.play_march(self.march_step % 2 == 1);
        }
    }

    fn alive_aliens(&self) -> usize {
        self.aliens.iter().filter(|alien| alien.alive).count()
    }

    fn enemy_fire_interval(&self) -> f32 {
        let total = (config::ALIEN_ROWS * config::ALIEN_COLS) as f32;
        let alive = self.alive_aliens() as f32;
        let tension = 1.0 - alive / total;
        let mut interval =
            (config::ENEMY_FIRE_BASE_INTERVAL - tension * 0.72 - self.wave as f32 * 0.05)
                .max(config::ENEMY_FIRE_MIN_INTERVAL);
        if self
            .shots
            .iter()
            .any(|shot| !shot.from_player && shot.kind == ShotKind::EnemyBomb)
        {
            interval *= config::BOMB_MIN_INTERVAL_SCALE;
        }
        interval
    }

    fn alien_rect(&self, alien: Alien) -> Rect {
        if alien.diving {
            return self.diving_alien_rect(alien);
        }
        Rect::new(
            self.formation_x + alien.col as f32 * config::ALIEN_SPACING_X
                - config::ALIEN_SIZE.x * 0.5,
            self.formation_y + alien.row as f32 * config::ALIEN_SPACING_Y
                - config::ALIEN_SIZE.y * 0.5,
            config::ALIEN_SIZE.x,
            config::ALIEN_SIZE.y,
        )
    }

    fn diving_alien_rect(&self, alien: Alien) -> Rect {
        Rect::new(
            alien.dive_pos.x - config::ALIEN_SIZE.x * 0.6,
            alien.dive_pos.y - config::ALIEN_SIZE.y * 0.75,
            config::ALIEN_SIZE.x * 1.2,
            config::ALIEN_SIZE.y * 1.5,
        )
    }

    fn player_rect(&self) -> Rect {
        Rect::new(
            self.player.x - config::PLAYER_WIDTH * 0.5,
            config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET - config::PLAYER_HEIGHT * 0.5,
            config::PLAYER_WIDTH,
            config::PLAYER_HEIGHT,
        )
    }

    fn alien_bounds(&self) -> (f32, f32, f32) {
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        let mut bottom = f32::MIN;
        let mut found = false;
        for alien in &self.aliens {
            if !alien.alive || alien.diving {
                continue;
            }
            found = true;
            let rect = self.alien_rect(*alien);
            left = left.min(rect.x);
            right = right.max(rect.x + rect.w);
            bottom = bottom.max(rect.y + rect.h);
        }
        if !found {
            return (0.0, 0.0, 0.0);
        }
        (left, right, bottom)
    }

    fn invaders_reached_player_zone(&self) -> bool {
        let (_, _, bottom) = self.alien_bounds();
        bottom >= config::BUNKER_Y - config::INVADER_REACH_MARGIN
    }

    fn explode_bomb(&mut self, center: Vec2) {
        self.screen_shake = self.screen_shake.max(18.0);
        self.play_enemy_explosion_sound();
        self.spawn_radial_burst(center, 28, config::ACCENT_C, 260.0, 1.0);
        self.spawn_radial_burst(center, 18, config::ACCENT_B, 180.0, 0.85);
        self.blast_waves.push(BlastWave {
            center,
            radius: 10.0,
            hit_player: false,
        });
    }

    fn player_shielded_from_bomb(&self, center: Vec2, player_rect: Rect) -> bool {
        self.bunkers
            .iter()
            .any(|bunker| bunker.blocks_bomb(center, player_rect))
    }

    fn player_protected_by_bunker(&self) -> bool {
        let player_rect = self.player_rect();
        self.bunkers
            .iter()
            .any(|bunker| bunker.blocks_player(player_rect))
    }
}

impl Bunker {
    fn damage_at_rect(&mut self, rect: Rect, strong: bool) -> bool {
        let mut hit = false;
        for row in 0..config::BUNKER_GRID_H {
            for col in 0..config::BUNKER_GRID_W {
                if self.cells[row][col] == 0 {
                    continue;
                }
                let cell_rect = Rect::new(
                    self.origin.x + col as f32 * config::BUNKER_CELL,
                    self.origin.y + row as f32 * config::BUNKER_CELL,
                    config::BUNKER_CELL - 1.0,
                    config::BUNKER_CELL - 1.0,
                );
                if rect.overlaps(&cell_rect) {
                    self.cells[row][col] =
                        self.cells[row][col].saturating_sub(if strong { 2 } else { 1 });
                    if !strong && row + 1 < config::BUNKER_GRID_H {
                        self.cells[row + 1][col] = self.cells[row + 1][col].saturating_sub(1);
                    }
                    hit = true;
                }
            }
        }
        hit
    }

    fn blocks_bomb(&self, center: Vec2, player_rect: Rect) -> bool {
        let min_x = player_rect.x;
        let max_x = player_rect.x + player_rect.w;
        let min_y = center.y.min(player_rect.center().y);
        let max_y = center.y.max(player_rect.center().y);

        for row in 0..config::BUNKER_GRID_H {
            for col in 0..config::BUNKER_GRID_W {
                if self.cells[row][col] == 0 {
                    continue;
                }
                let cell_rect = Rect::new(
                    self.origin.x + col as f32 * config::BUNKER_CELL,
                    self.origin.y + row as f32 * config::BUNKER_CELL,
                    config::BUNKER_CELL - 1.0,
                    config::BUNKER_CELL - 1.0,
                );
                if cell_rect.x <= max_x
                    && cell_rect.x + cell_rect.w >= min_x
                    && cell_rect.y <= max_y
                    && cell_rect.y + cell_rect.h >= min_y
                {
                    return true;
                }
            }
        }
        false
    }

    fn blocks_player(&self, player_rect: Rect) -> bool {
        let min_x = player_rect.x;
        let max_x = player_rect.x + player_rect.w;
        let max_y = player_rect.center().y;

        for row in 0..config::BUNKER_GRID_H {
            for col in 0..config::BUNKER_GRID_W {
                if self.cells[row][col] == 0 {
                    continue;
                }
                let cell_rect = Rect::new(
                    self.origin.x + col as f32 * config::BUNKER_CELL,
                    self.origin.y + row as f32 * config::BUNKER_CELL,
                    config::BUNKER_CELL - 1.0,
                    config::BUNKER_CELL - 1.0,
                );
                if cell_rect.x <= max_x
                    && cell_rect.x + cell_rect.w >= min_x
                    && cell_rect.y <= max_y
                {
                    return true;
                }
            }
        }
        false
    }

    fn draw(&self, offset: Vec2) {
        for row in 0..config::BUNKER_GRID_H {
            for col in 0..config::BUNKER_GRID_W {
                let hp = self.cells[row][col];
                if hp == 0 {
                    continue;
                }
                let x = self.origin.x + col as f32 * config::BUNKER_CELL + offset.x;
                let y = self.origin.y + row as f32 * config::BUNKER_CELL + offset.y;
                let base = match hp {
                    3 => config::BUNKER_COLOR,
                    2 => Color::from_rgba(151, 239, 111, 255),
                    _ => Color::from_rgba(255, 166, 110, 255),
                };
                let bevel = mix_color(base, WHITE, 0.22);
                let shadow = mix_color(base, BLACK, 0.32);
                let side_facet = mix_color(base, BLACK, 0.16);
                let top_facet = mix_color(base, WHITE, 0.34);
                draw_rectangle(
                    x,
                    y,
                    config::BUNKER_CELL - 1.5,
                    config::BUNKER_CELL - 1.5,
                    base,
                );
                draw_rectangle(
                    x + 2.0,
                    y + 5.0,
                    config::BUNKER_CELL - 4.5,
                    config::BUNKER_CELL - 7.0,
                    Color::new(shadow.r, shadow.g, shadow.b, 0.85),
                );
                draw_rectangle(
                    x + config::BUNKER_CELL - 6.0,
                    y + 2.0,
                    4.0,
                    config::BUNKER_CELL - 6.0,
                    Color::new(side_facet.r, side_facet.g, side_facet.b, 0.95),
                );
                draw_rectangle(
                    x + 1.0,
                    y + 1.0,
                    config::BUNKER_CELL - 4.0,
                    3.0,
                    Color::new(bevel.r, bevel.g, bevel.b, 0.82),
                );
                draw_rectangle(
                    x + 2.0,
                    y + 2.0,
                    config::BUNKER_CELL - 7.0,
                    2.0,
                    Color::new(top_facet.r, top_facet.g, top_facet.b, 0.9),
                );
                draw_rectangle(
                    x - 1.0,
                    y - 1.0,
                    config::BUNKER_CELL + 0.5,
                    config::BUNKER_CELL + 0.5,
                    Color::new(base.r, base.g, base.b, 0.12),
                );
            }
        }
    }
}

fn build_aliens() -> Vec<Alien> {
    let mut aliens = Vec::with_capacity(config::ALIEN_ROWS * config::ALIEN_COLS);
    for row in 0..config::ALIEN_ROWS {
        for col in 0..config::ALIEN_COLS {
            aliens.push(Alien {
                row,
                col,
                alive: true,
                frame: false,
                fire_flash: 0.0,
                diving: false,
                dive_pos: Vec2::ZERO,
                dive_vel: Vec2::ZERO,
                dive_angle: 0.0,
            });
        }
    }
    aliens
}

fn build_bunkers() -> Vec<Bunker> {
    let mut bunkers = Vec::with_capacity(config::BUNKER_COUNT);
    let spacing = (config::WINDOW_WIDTH - 360.0) / (config::BUNKER_COUNT - 1) as f32;
    for i in 0..config::BUNKER_COUNT {
        let mut cells = [[0u8; config::BUNKER_GRID_W]; config::BUNKER_GRID_H];
        for (row, row_cells) in cells.iter_mut().enumerate() {
            for (col, cell) in row_cells.iter_mut().enumerate() {
                let arch_cut = row >= 4 && (4..=5).contains(&col);
                let rounded = (row == 0 && (col <= 1 || col >= config::BUNKER_GRID_W - 2))
                    || (row == 1 && (col == 0 || col == config::BUNKER_GRID_W - 1));
                if !arch_cut && !rounded {
                    *cell = 3;
                }
            }
        }
        bunkers.push(Bunker {
            origin: vec2(180.0 + i as f32 * spacing, config::BUNKER_Y),
            cells,
        });
    }
    bunkers
}

fn build_stars() -> Vec<Vec<Star>> {
    let mut layers = Vec::with_capacity(config::STAR_LAYER_COUNT);
    for layer in 0..config::STAR_LAYER_COUNT {
        let mut stars = Vec::with_capacity(config::STARS_PER_LAYER);
        for _ in 0..config::STARS_PER_LAYER {
            stars.push(Star {
                pos: vec2(
                    rand::gen_range(0.0, config::WINDOW_WIDTH),
                    rand::gen_range(0.0, config::WINDOW_HEIGHT),
                ),
                speed: 8.0 + layer as f32 * 12.0 + rand::gen_range(0.0, 8.0),
                radius: 0.8 + layer as f32 * 0.6 + rand::gen_range(0.0, 1.1),
                alpha: 0.3 + layer as f32 * 0.18 + rand::gen_range(0.0, 0.2),
            });
        }
        layers.push(stars);
    }
    layers
}

fn draw_shot(shot: Shot, offset: Vec2) {
    let pos = shot.pos + offset;
    match shot.kind {
        ShotKind::Bolt => {
            let color = if shot.from_player {
                config::PLAYER_SHOT_COLOR
            } else {
                config::ENEMY_SHOT_COLOR
            };
            let core = mix_color(color, WHITE, 0.35);
            let flicker = (get_time() as f32 * 18.0 + shot.pos.y * 0.06).sin() * 0.5 + 0.5;
            let glow = Color::new(
                color.r,
                color.g,
                color.b,
                if shot.from_player {
                    0.22
                } else {
                    0.28 + flicker * 0.14
                },
            );
            if !shot.from_player {
                let trail_h = shot.size.y * (2.3 + flicker * 1.2);
                draw_rectangle(
                    pos.x - shot.size.x * 1.4,
                    pos.y - trail_h * 0.9,
                    shot.size.x * 2.8,
                    trail_h,
                    Color::new(color.r, color.g, color.b, 0.10 + flicker * 0.08),
                );
                draw_rectangle(
                    pos.x - shot.size.x * 0.9,
                    pos.y - trail_h * 0.68,
                    shot.size.x * 1.8,
                    trail_h * 0.62,
                    Color::from_rgba(255, 220, 150, ((70.0 + flicker * 60.0).round()) as u8),
                );
            }
            draw_rectangle(
                pos.x - shot.size.x * 1.2,
                pos.y - shot.size.y * 0.5,
                shot.size.x * 2.4,
                shot.size.y,
                glow,
            );
            draw_rectangle(
                pos.x - shot.size.x * 0.5,
                pos.y - shot.size.y * 0.5,
                shot.size.x,
                shot.size.y,
                color,
            );
            draw_rectangle(
                pos.x - shot.size.x * 0.22,
                pos.y - shot.size.y * 0.42,
                shot.size.x * 0.44,
                shot.size.y * 0.84,
                core,
            );
        }
        ShotKind::EnemyBomb => {
            let pulse = (get_time() as f32 * 10.0 + shot.pos.y * 0.04).sin() * 0.5 + 0.5;
            draw_circle(
                pos.x,
                pos.y,
                shot.size.x * (1.65 + pulse * 0.2),
                Color::from_rgba(255, 82, 72, (34.0 + pulse * 22.0).round() as u8),
            );
            draw_circle(
                pos.x,
                pos.y,
                shot.size.x * 1.15,
                Color::from_rgba(255, 214, 92, (46.0 + pulse * 18.0).round() as u8),
            );
            draw_circle(pos.x, pos.y, shot.size.x * 0.65, config::ACCENT_C);
            draw_circle(pos.x, pos.y, shot.size.x * 0.3, WHITE);
            draw_line(
                pos.x,
                pos.y - shot.size.y * 0.5,
                pos.x,
                pos.y - shot.size.y * (2.2 + pulse * 0.35),
                7.0,
                Color::from_rgba(255, 118, 76, (70.0 + pulse * 40.0).round() as u8),
            );
            draw_line(
                pos.x,
                pos.y - shot.size.y,
                pos.x,
                pos.y - shot.size.y * 1.55,
                4.0,
                config::ACCENT_B,
            );
        }
        ShotKind::PlayerBomb => {
            draw_circle(
                pos.x,
                pos.y,
                shot.size.x * 1.2,
                Color::from_rgba(255, 214, 92, 42),
            );
            draw_circle(pos.x, pos.y, shot.size.x * 0.72, config::PLAYER_BOMB_COLOR);
            draw_circle(pos.x, pos.y, shot.size.x * 0.34, WHITE);
            draw_line(
                pos.x,
                pos.y + shot.size.y * 0.8,
                pos.x,
                pos.y + shot.size.y * 1.45,
                4.0,
                config::ACCENT_A,
            );
        }
    }
}

fn shot_rect(shot: Shot) -> Rect {
    Rect::new(
        shot.pos.x - shot.size.x * 0.5,
        shot.pos.y - shot.size.y * 0.5,
        shot.size.x,
        shot.size.y,
    )
}

fn alien_score(row: usize) -> u32 {
    match row {
        0 => 40,
        1 | 2 => 30,
        _ => 20,
    }
}

fn alien_color(row: usize) -> Color {
    config::ALIEN_ROW_COLORS[row]
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn rotated_point(point: Vec2, angle: f32) -> Vec2 {
    let (sin_a, cos_a) = angle.sin_cos();
    vec2(
        point.x * cos_a - point.y * sin_a,
        point.x * sin_a + point.y * cos_a,
    )
}

fn arcade_text(text: &str, x: f32, y: f32, font_size: f32, color: Color, centered: bool) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    let draw_x = if centered { x - dims.width * 0.5 } else { x };
    for (dx, dy) in [(-3.0, 0.0), (3.0, 0.0), (0.0, -3.0), (0.0, 3.0)] {
        draw_text_ex(
            text,
            draw_x + dx,
            y + dy,
            TextParams {
                font_size: font_size as u16,
                color: Color::from_rgba(12, 8, 20, 255),
                ..Default::default()
            },
        );
    }
    draw_text_ex(
        text,
        draw_x,
        y + 2.0,
        TextParams {
            font_size: font_size as u16,
            color: mix_color(color, BLACK, 0.35),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        draw_x,
        y,
        TextParams {
            font_size: font_size as u16,
            color,
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        draw_x,
        y - 2.0,
        TextParams {
            font_size: font_size as u16,
            color: Color::new(1.0, 1.0, 1.0, 0.32),
            ..Default::default()
        },
    );
}

fn arcade_text_centered(text: &str, x: f32, y: f32, font_size: f32, color: Color) {
    arcade_text(text, x, y, font_size, color, true);
}

fn arcade_title(text: &str, x: f32, y: f32, font_size: f32, color: Color, centered: bool) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    let draw_x = if centered { x - dims.width * 0.5 } else { x };
    draw_text_ex(
        text,
        draw_x,
        y + 10.0,
        TextParams {
            font_size: font_size as u16,
            color: Color::from_rgba(8, 4, 16, 255),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        draw_x - 4.0,
        y,
        TextParams {
            font_size: font_size as u16,
            color: mix_color(color, BLACK, 0.35),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        draw_x + 4.0,
        y,
        TextParams {
            font_size: font_size as u16,
            color: mix_color(color, WHITE, 0.15),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        draw_x,
        y,
        TextParams {
            font_size: font_size as u16,
            color,
            ..Default::default()
        },
    );
}

fn draw_arcade_panel(x: f32, y: f32, w: f32, h: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::from_rgba(7, 8, 24, 190));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(255, 255, 255, 30));
    draw_rectangle_lines(
        x + 4.0,
        y + 4.0,
        w - 8.0,
        h - 8.0,
        2.0,
        Color::new(color.r, color.g, color.b, 0.75),
    );
}

fn high_score_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("neon-invaders").join("highscore.txt"))
}

fn load_high_score() -> u32 {
    let Some(path) = high_score_path() else {
        return 0;
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

fn save_high_score(score: u32) {
    let Some(path) = high_score_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, score.to_string());
}
