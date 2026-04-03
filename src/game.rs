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
    dive_target_x: f32,
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

#[derive(Clone)]
struct PlayerProfile {
    name: String,
    high_score: u32,
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
    profiles: Vec<PlayerProfile>,
    current_player_idx: usize,
    entering_name: bool,
    name_input: String,
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
        let profiles = load_profiles();
        let current_player_idx = 0;
        let high_score = profiles
            .get(current_player_idx)
            .map(|profile| profile.high_score)
            .unwrap_or(0);
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
            formation_x: formation_start_x(),
            formation_y: config::ALIEN_START_Y,
            alien_dir: 1.0,
            enemy_fire_timer: config::ENEMY_FIRE_BASE_INTERVAL,
            bunkers: Vec::new(),
            particles: Vec::with_capacity(config::PARTICLE_CAP),
            blast_waves: Vec::new(),
            stars: Vec::new(),
            profiles,
            current_player_idx,
            entering_name: false,
            name_input: String::new(),
            score: 0,
            high_score,
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
        if app.profiles.is_empty() {
            app.entering_name = true;
        }
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

        if let Some(profile) = self.profiles.get_mut(self.current_player_idx)
            && self.score > profile.high_score
        {
            profile.high_score = self.score;
            self.high_score = self.score;
            save_profiles(&self.profiles);
        }
    }

    pub fn draw(&self) {
        clear_background(Color::from_rgba(1, 2, 8, 255));
        let (viewport, scale) = self.viewport_frame();
        self.draw_letterbox_backdrop(viewport, scale);

        let mut camera = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            config::WINDOW_WIDTH,
            config::WINDOW_HEIGHT,
        ));
        camera.viewport = Some((
            viewport.x.round() as i32,
            viewport.y.round() as i32,
            (config::WINDOW_WIDTH * scale).round() as i32,
            (config::WINDOW_HEIGHT * scale).round() as i32,
        ));
        set_camera(&camera);
        self.draw_background(Vec2::ZERO);

        let shake = if self.screen_shake > 0.0 {
            vec2(
                rand::gen_range(-self.screen_shake, self.screen_shake),
                rand::gen_range(-self.screen_shake, self.screen_shake),
            )
        } else {
            Vec2::ZERO
        };

        self.draw_playfield(shake);
        self.draw_hud(Vec2::ZERO);
        self.draw_overlay(Vec2::ZERO);
        set_default_camera();
        self.draw_viewport_frame(viewport, scale);
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
        if self.entering_name {
            while let Some(ch) = get_char_pressed() {
                if (ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_')
                    && self.name_input.len() < 16
                {
                    self.name_input.push(ch);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.name_input.pop();
            }
            if is_key_pressed(KeyCode::Enter) {
                let name = self.name_input.trim();
                if !name.is_empty() {
                    if let Some(index) = self
                        .profiles
                        .iter()
                        .position(|profile| profile.name.eq_ignore_ascii_case(name))
                    {
                        self.current_player_idx = index;
                    } else {
                        let profile = PlayerProfile {
                            name: name.to_string(),
                            high_score: 0,
                        };
                        self.profiles.push(profile);
                        self.profiles.sort_by(|a, b| {
                            b.high_score.cmp(&a.high_score).then(a.name.cmp(&b.name))
                        });
                        self.current_player_idx = self
                            .profiles
                            .iter()
                            .position(|profile| profile.name.eq_ignore_ascii_case(name))
                            .unwrap_or(0);
                    }
                    self.high_score = self.profiles[self.current_player_idx].high_score;
                    self.entering_name = false;
                    self.name_input.clear();
                    save_profiles(&self.profiles);
                }
            }
            return;
        }
        if !self.profiles.is_empty() {
            if is_key_pressed(KeyCode::Up) {
                if self.current_player_idx == 0 {
                    self.current_player_idx = self.profiles.len() - 1;
                } else {
                    self.current_player_idx -= 1;
                }
                self.high_score = self.profiles[self.current_player_idx].high_score;
            }
            if is_key_pressed(KeyCode::Down) {
                self.current_player_idx = (self.current_player_idx + 1) % self.profiles.len();
                self.high_score = self.profiles[self.current_player_idx].high_score;
            }
        }
        if is_key_pressed(KeyCode::N) {
            self.entering_name = true;
            self.name_input.clear();
            return;
        }
        if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
            && !self.profiles.is_empty()
        {
            self.pending_restart = false;
            self.high_score = self.profiles[self.current_player_idx].high_score;
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
            .clamp(half_w + 28.0, config::WINDOW_WIDTH - half_w - 28.0);

        if is_key_down(KeyCode::Space)
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
        let tension = 1.0 - alive as f32 / (config::ALIEN_ROWS * config::ALIEN_COLS) as f32;
        let speed_scale = 1.0 + tension * 2.7 + tension.powf(2.0) * 0.6;
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
            let mut bunker_impact = None;

            for bunker in &mut self.bunkers {
                if bunker_impact.is_none() {
                    bunker_impact = bunker.impact_point(shot_rect.center());
                }
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
                self.spawn_bunker_debris(
                    bunker_impact.unwrap_or_else(|| shot_rect.center()),
                    if shot.from_player {
                        config::PLAYER_SHOT_COLOR
                    } else {
                        config::ENEMY_SHOT_COLOR
                    },
                );
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
        let floor_y = config::PLAYER_ZONE_Y + offset.y;
        draw_glow_circle(
            config::WINDOW_WIDTH * 0.5 + offset.x,
            floor_y + 14.0,
            280.0,
            Color::new(
                config::FLOOR_GLOW.r,
                config::FLOOR_GLOW.g,
                config::FLOOR_GLOW.b,
                0.10,
            ),
        );
        draw_rectangle(
            72.0 + offset.x,
            floor_y - 8.0,
            config::WINDOW_WIDTH - 144.0,
            30.0,
            Color::from_rgba(18, 28, 54, 72),
        );

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
            let trail = particle.pos - particle.vel.normalize_or_zero() * particle.size * 2.6;
            draw_line(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                trail.x + offset.x,
                trail.y + offset.y,
                particle.size * (0.55 + t * 0.35),
                Color::new(
                    particle.color.r,
                    particle.color.g,
                    particle.color.b,
                    0.22 * t,
                ),
            );
            draw_circle(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                particle.size * (1.8 - t),
                glow,
            );
            draw_poly(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                4,
                particle.size * (0.35 + t * 0.3),
                get_time() as f32 * 140.0 + particle.pos.x * 0.2,
                Color::new(
                    particle.color.r,
                    particle.color.g,
                    particle.color.b,
                    0.18 * t,
                ),
            );
            draw_circle(
                particle.pos.x + offset.x,
                particle.pos.y + offset.y,
                particle.size * (0.55 + t * 0.45),
                Color::new(particle.color.r, particle.color.g, particle.color.b, t),
            );
            draw_line(
                particle.pos.x - particle.size * 0.7 + offset.x,
                particle.pos.y + offset.y,
                particle.pos.x + particle.size * 0.7 + offset.x,
                particle.pos.y + offset.y,
                1.0,
                Color::new(1.0, 1.0, 1.0, 0.18 * t),
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
            72.0 + offset.x,
            floor_y,
            config::WINDOW_WIDTH - 72.0 + offset.x,
            floor_y,
            3.0,
            Color::from_rgba(255, 255, 255, 34),
        );
        draw_line(
            92.0 + offset.x,
            floor_y + 5.0,
            config::WINDOW_WIDTH - 92.0 + offset.x,
            floor_y + 5.0,
            1.0,
            Color::new(
                config::FLOOR_GLOW.r,
                config::FLOOR_GLOW.g,
                config::FLOOR_GLOW.b,
                0.58,
            ),
        );
    }

    fn draw_hud(&self, offset: Vec2) {
        let ox = offset.x;
        let oy = offset.y;
        draw_rectangle(
            ox,
            oy,
            config::WINDOW_WIDTH,
            118.0,
            Color::from_rgba(3, 6, 18, 56),
        );
        draw_line(
            ox + 36.0,
            oy + 92.0,
            ox + config::WINDOW_WIDTH - 36.0,
            oy + 92.0,
            1.0,
            Color::from_rgba(255, 255, 255, 18),
        );
        draw_hud_card(ox + 28.0, oy + 18.0, 362.0, 70.0, config::ACCENT_A);
        draw_hud_card(
            ox + config::WINDOW_WIDTH * 0.5 - 188.0,
            oy + 18.0,
            376.0,
            70.0,
            config::ACCENT_C,
        );
        draw_hud_card(
            ox + config::WINDOW_WIDTH - 438.0,
            oy + 18.0,
            202.0,
            70.0,
            config::ACCENT_C,
        );
        draw_hud_card(
            ox + config::WINDOW_WIDTH - 224.0,
            oy + 18.0,
            196.0,
            70.0,
            config::ACCENT_A,
        );

        arcade_text(
            "SCORE",
            ox + 46.0,
            oy + 42.0,
            16.0,
            config::HUD_COLOR,
            false,
        );
        arcade_text(
            &format!("{:06}", self.score),
            ox + 44.0,
            oy + 74.0,
            34.0,
            config::ACCENT_A,
            false,
        );
        if let Some(profile) = self.profiles.get(self.current_player_idx) {
            arcade_text(
                &format!("PILOT {}", profile.name.to_uppercase()),
                ox + 46.0,
                oy + 108.0,
                18.0,
                config::HUD_COLOR,
                false,
            );
        }

        arcade_text_centered(
            &format!("WAVE {}", self.wave),
            ox + config::WINDOW_WIDTH * 0.5,
            oy + 46.0,
            18.0,
            config::HUD_COLOR,
        );
        arcade_text_centered(
            &format!("{:02}", self.wave),
            ox + config::WINDOW_WIDTH * 0.5,
            oy + 76.0,
            36.0,
            config::ACCENT_C,
        );
        arcade_text_centered(
            &format!("HIGH {:06}", self.high_score),
            ox + config::WINDOW_WIDTH * 0.5,
            oy + 108.0,
            18.0,
            config::HUD_COLOR,
        );
        if self.rapid_fire_timer > 0.0 {
            arcade_text_centered(
                &format!("RAPID FIRE {:.1}", self.rapid_fire_timer),
                ox + config::WINDOW_WIDTH * 0.5,
                oy + 132.0,
                20.0,
                config::ACCENT_B,
            );
        }

        let bombs_x = ox + config::WINDOW_WIDTH - 418.0;
        arcade_text(
            "BOMBS",
            bombs_x + 16.0,
            oy + 44.0,
            16.0,
            config::HUD_COLOR,
            false,
        );
        for i in 0..self.player.bombs.min(6) {
            let x = bombs_x + 22.0 + i as f32 * 28.0;
            let y = oy + 66.0;
            draw_glow_circle(x, y, 18.0, Color::from_rgba(255, 170, 82, 44));
            draw_circle(x, y, 9.0, config::PLAYER_BOMB_COLOR);
            draw_circle(x, y, 4.2, WHITE);
            draw_line(
                x,
                y - 9.0,
                x,
                y - 18.0,
                3.0,
                Color::from_rgba(255, 228, 132, 255),
            );
        }
        if self.player.bombs > 6 {
            arcade_text(
                &format!("+{}", self.player.bombs - 6),
                bombs_x + 22.0 + 6.0 * 28.0,
                oy + 74.0,
                20.0,
                config::ACCENT_C,
                false,
            );
        }

        let lives_y = oy + 66.0;
        arcade_text(
            "LIVES",
            ox + config::WINDOW_WIDTH - 204.0,
            oy + 44.0,
            16.0,
            config::HUD_COLOR,
            false,
        );
        for i in 0..self.lives {
            let x = ox + config::WINDOW_WIDTH - 184.0 + i as f32 * 28.0;
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
    }

    fn draw_overlay(&self, offset: Vec2) {
        let ox = offset.x;
        let oy = offset.y;
        match self.screen {
            ScreenState::Title => {
                let fade = (self.state_timer / config::TITLE_FADE_TIME).min(1.0);
                draw_rectangle(
                    0.0,
                    0.0,
                    config::WINDOW_WIDTH,
                    config::WINDOW_HEIGHT,
                    Color::new(0.02, 0.03, 0.08, 0.68 * fade),
                );
                draw_holo_frame(
                    ox + config::WINDOW_WIDTH * 0.5 - 470.0,
                    oy + 126.0,
                    940.0,
                    660.0,
                    config::ACCENT_A,
                );
                arcade_title(
                    "NEON INVADERS",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 218.0,
                    90.0,
                    config::ACCENT_A,
                    true,
                );
                arcade_text_centered(
                    "Classic invasion. Modern arcade energy.",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 286.0,
                    24.0,
                    config::ACCENT_C,
                );
                if self.entering_name {
                    draw_hud_card(
                        ox + config::WINDOW_WIDTH * 0.5 - 300.0,
                        oy + 356.0,
                        600.0,
                        168.0,
                        config::ACCENT_A,
                    );
                    arcade_text_centered(
                        "ENTER PILOT NAME",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 402.0,
                        32.0,
                        config::ACCENT_A,
                    );
                    arcade_text_centered(
                        &format!("{}_", self.name_input.to_uppercase()),
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 454.0,
                        34.0,
                        config::ACCENT_C,
                    );
                    arcade_text_centered(
                        "Type name, then press Enter",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 496.0,
                        22.0,
                        config::HUD_COLOR,
                    );
                } else {
                    draw_hud_card(
                        ox + config::WINDOW_WIDTH * 0.5 - 352.0,
                        oy + 334.0,
                        704.0,
                        250.0,
                        config::ACCENT_B,
                    );
                    arcade_text_centered(
                        "SELECT PILOT",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 388.0,
                        28.0,
                        config::ACCENT_B,
                    );
                    for (i, profile) in self.profiles.iter().take(6).enumerate() {
                        let y = oy + 432.0 + i as f32 * 32.0;
                        let selected = i == self.current_player_idx;
                        arcade_text_centered(
                            &format!(
                                "{}   {:06}",
                                profile.name.to_uppercase(),
                                profile.high_score
                            ),
                            ox + config::WINDOW_WIDTH * 0.5,
                            y,
                            24.0,
                            if selected {
                                config::ACCENT_C
                            } else {
                                config::HUD_COLOR
                            },
                        );
                    }
                    if let Some(profile) = self.profiles.get(self.current_player_idx) {
                        arcade_text_centered(
                            &format!("BEST {:06}", profile.high_score),
                            ox + config::WINDOW_WIDTH * 0.5,
                            oy + 618.0,
                            22.0,
                            config::ACCENT_C,
                        );
                    }
                    arcade_text_centered(
                        "Up/Down: Select Pilot    N: New Pilot",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 654.0,
                        20.0,
                        config::HUD_COLOR,
                    );
                    arcade_text_centered(
                        "Move: A / D or Left / Right    Shot: Space    Bomb: Up    Quit: Esc",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 694.0,
                        20.0,
                        config::HUD_COLOR,
                    );
                    arcade_text_centered(
                        "Press Space to Start",
                        ox + config::WINDOW_WIDTH * 0.5,
                        oy + 748.0 + self.state_timer.sin() * 8.0,
                        32.0,
                        config::ACCENT_C,
                    );
                }
            }
            ScreenState::WaveIntro => {
                let alpha = (1.0 - self.state_timer / config::WAVE_INTRO_TIME).clamp(0.0, 1.0);
                draw_hud_card(
                    ox + config::WINDOW_WIDTH * 0.5 - 250.0,
                    oy + 288.0,
                    500.0,
                    140.0,
                    Color::new(
                        config::ACCENT_A.r,
                        config::ACCENT_A.g,
                        config::ACCENT_A.b,
                        alpha,
                    ),
                );
                arcade_text_centered(
                    &format!("WAVE {}", self.wave),
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 346.0,
                    58.0,
                    Color::new(
                        config::ACCENT_A.r,
                        config::ACCENT_A.g,
                        config::ACCENT_A.b,
                        alpha,
                    ),
                );
                arcade_text_centered(
                    "Formation incoming",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 396.0,
                    22.0,
                    Color::new(
                        config::HUD_COLOR.r,
                        config::HUD_COLOR.g,
                        config::HUD_COLOR.b,
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
                draw_holo_frame(
                    ox + config::WINDOW_WIDTH * 0.5 - 260.0,
                    oy + 250.0,
                    520.0,
                    300.0,
                    config::ACCENT_A,
                );
                arcade_title(
                    "PAUSED",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 340.0,
                    84.0,
                    config::ACCENT_A,
                    true,
                );
                arcade_text_centered(
                    "Press Space to Resume",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 420.0,
                    30.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Press Esc to Return to Title",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 470.0,
                    28.0,
                    config::HUD_COLOR,
                );
            }
            ScreenState::WaveClear => {
                draw_hud_card(
                    ox + config::WINDOW_WIDTH * 0.5 - 290.0,
                    oy + 270.0,
                    580.0,
                    170.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "WAVE CLEARED",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 340.0,
                    54.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Incoming formation detected...",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 394.0,
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
                draw_holo_frame(
                    ox + config::WINDOW_WIDTH * 0.5 - 310.0,
                    oy + 232.0,
                    620.0,
                    340.0,
                    config::ACCENT_B,
                );
                arcade_title(
                    "GAME OVER",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 320.0,
                    92.0,
                    config::ACCENT_B,
                    true,
                );
                arcade_text_centered(
                    &format!("Final Score {:06}", self.score),
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 405.0,
                    32.0,
                    config::ACCENT_C,
                );
                arcade_text_centered(
                    "Press Space to Restart or Esc for Title",
                    ox + config::WINDOW_WIDTH * 0.5,
                    oy + 500.0,
                    28.0,
                    config::HUD_COLOR,
                );
            }
            ScreenState::Playing => {}
        }
    }

    fn draw_background(&self, offset: Vec2) {
        let ox = offset.x;
        let oy = offset.y;
        draw_rectangle(
            0.0,
            0.0,
            config::WINDOW_WIDTH,
            config::WINDOW_HEIGHT,
            config::BG_TOP,
        );
        for i in 0..10 {
            let y = oy + i as f32 / 10.0 * config::WINDOW_HEIGHT;
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
        draw_glow_circle(
            ox + 270.0,
            oy + 136.0,
            190.0,
            Color::new(
                config::BG_GLOW_B.r,
                config::BG_GLOW_B.g,
                config::BG_GLOW_B.b,
                0.06,
            ),
        );
        draw_glow_circle(
            ox + 730.0,
            oy + 200.0,
            240.0,
            Color::new(
                config::BG_GLOW_A.r,
                config::BG_GLOW_A.g,
                config::BG_GLOW_A.b,
                0.05,
            ),
        );
        draw_glow_circle(
            ox + 1260.0,
            oy + 170.0,
            220.0,
            Color::new(
                config::PLAYER_COLOR.r,
                config::PLAYER_COLOR.g,
                config::PLAYER_COLOR.b,
                0.04,
            ),
        );
        draw_glow_circle(
            ox + 1700.0,
            oy + 220.0,
            280.0,
            Color::new(
                config::BG_GLOW_C.r,
                config::BG_GLOW_C.g,
                config::BG_GLOW_C.b,
                0.04,
            ),
        );

        for (layer, stars) in self.stars.iter().enumerate() {
            let tint = match layer {
                0 => WHITE,
                1 => config::ACCENT_C,
                _ => WHITE,
            };
            for star in stars {
                let glow = Color::new(tint.r, tint.g, tint.b, star.alpha * 0.12);
                draw_circle(star.pos.x + ox, star.pos.y + oy, star.radius * 2.1, glow);
                draw_circle(
                    star.pos.x + ox,
                    star.pos.y + oy,
                    star.radius,
                    Color::new(tint.r, tint.g, tint.b, star.alpha),
                );
            }
        }

        for x in [140.0, 460.0, 820.0, 1180.0, 1520.0, 1830.0] {
            draw_rectangle(
                ox + x,
                oy,
                1.5,
                config::WINDOW_HEIGHT,
                Color::from_rgba(255, 255, 255, 8),
            );
        }
        for (x, y, w, h, color) in [
            (86.0, 90.0, 300.0, 2.0, Color::from_rgba(82, 178, 255, 18)),
            (
                1450.0,
                144.0,
                240.0,
                2.0,
                Color::from_rgba(255, 110, 164, 18),
            ),
            (1180.0, 88.0, 190.0, 2.0, Color::from_rgba(255, 214, 92, 16)),
        ] {
            draw_rectangle(ox + x, oy + y, w, h, color);
        }
        draw_rectangle(
            ox,
            oy + config::WINDOW_HEIGHT - 240.0,
            config::WINDOW_WIDTH,
            240.0,
            Color::from_rgba(6, 8, 22, 96),
        );
        draw_glow_circle(
            ox + config::WINDOW_WIDTH * 0.5,
            oy + config::WINDOW_HEIGHT - 118.0,
            340.0,
            Color::new(
                config::FLOOR_GLOW.r,
                config::FLOOR_GLOW.g,
                config::FLOOR_GLOW.b,
                0.08,
            ),
        );
        draw_glow_circle(
            ox + config::WINDOW_WIDTH * 0.5,
            oy + config::WINDOW_HEIGHT - 66.0,
            620.0,
            Color::from_rgba(255, 255, 255, 8),
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
        let rim = mix_color(color, WHITE, 0.55);
        draw_ellipse(
            rect.center().x,
            rect.y + rect.h + 18.0,
            62.0,
            16.0,
            0.0,
            Color::from_rgba(4, 8, 20, 124),
        );
        draw_glow_circle(rect.center().x, rect.center().y + 6.0, 58.0, glow);
        draw_glow_circle(
            rect.center().x,
            rect.center().y + 10.0,
            32.0,
            Color::new(warm_panel.r, warm_panel.g, warm_panel.b, 0.12),
        );
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
            vec2(rect.x + rect.w * 0.5, rect.y - 22.0),
            vec2(rect.x + 22.0, rect.y + 6.0),
            vec2(rect.x + rect.w - 22.0, rect.y + 6.0),
            Color::new(panel.r, panel.g, panel.b, 0.72),
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
            rect.x + rect.w * 0.5 - 16.0,
            rect.y + 12.0,
            6.0,
            18.0,
            mix_color(cool_panel, WHITE, 0.2),
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 + 10.0,
            rect.y + 12.0,
            6.0,
            18.0,
            mix_color(cool_panel, BLACK, 0.1),
        );
        draw_rectangle(
            rect.x + rect.w * 0.5 - 8.0,
            rect.y + 24.0,
            16.0,
            10.0,
            cool_panel,
        );
        draw_line(
            rect.x + rect.w * 0.5,
            rect.y - 14.0,
            rect.x + rect.w * 0.5,
            rect.y + rect.h + 2.0,
            2.0,
            Color::new(rim.r, rim.g, rim.b, 0.65),
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
        draw_circle(
            rect.x + 26.0,
            rect.y + rect.h + 1.0,
            4.0,
            Color::from_rgba(255, 188, 92, 180),
        );
        draw_circle(
            rect.x + rect.w - 26.0,
            rect.y + rect.h + 1.0,
            4.0,
            Color::from_rgba(255, 188, 92, 180),
        );
        draw_line(
            rect.x + 12.0,
            rect.y + rect.h + 2.0,
            rect.x + rect.w - 12.0,
            rect.y + rect.h + 2.0,
            2.0,
            Color::from_rgba(255, 255, 255, 55),
        );
        draw_line(
            rect.x + 18.0,
            rect.y + 5.0,
            rect.x + rect.w - 18.0,
            rect.y + 5.0,
            1.5,
            Color::from_rgba(255, 255, 255, 65),
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
        draw_ellipse(
            center.x,
            center.y + 28.0,
            36.0,
            9.0,
            0.0,
            Color::from_rgba(3, 6, 18, 90),
        );
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
        draw_line(
            center.x - 18.0,
            center.y - 14.0,
            center.x + 18.0,
            center.y - 14.0,
            1.4,
            Color::new(1.0, 1.0, 1.0, 0.12 + sweep * 0.16 + fire_flash * 0.1),
        );
        draw_circle(
            center.x + 15.0,
            center.y + 5.0,
            9.0,
            Color::new(
                accent_alt.r,
                accent_alt.g,
                accent_alt.b,
                0.08 + sweep * 0.09,
            ),
        );
    }

    fn draw_diving_alien(&self, alien: Alien, offset: Vec2) {
        let center = alien.dive_pos + offset;
        let color = alien_color(alien.row);
        let accent = mix_color(color, config::ACCENT_B, 0.45);
        let canopy = mix_color(color, WHITE, 0.65);
        let phase = alien.dive_angle;
        let width_scale = phase.cos().abs().max(0.16);
        let facing = if phase.cos() >= 0.0 { 1.0 } else { -1.0 };
        let wing_depth = phase.sin() * 9.0;
        let body_half = 19.0 * width_scale;
        let wing_span = 30.0 * width_scale;
        let belly_half = 14.0 * width_scale;
        let glow = Color::new(color.r, color.g, color.b, 0.24 + alien.fire_flash * 0.16);
        draw_ellipse(
            center.x,
            center.y + 28.0,
            34.0,
            8.0,
            0.0,
            Color::from_rgba(4, 8, 20, 84),
        );
        draw_glow_circle(center.x, center.y, 42.0, glow);

        draw_triangle(
            vec2(center.x, center.y - 24.0),
            vec2(center.x - body_half, center.y + 8.0),
            vec2(center.x + body_half, center.y + 8.0),
            color,
        );
        draw_triangle(
            vec2(center.x, center.y - 8.0),
            vec2(center.x - belly_half, center.y + 11.0),
            vec2(center.x + belly_half, center.y + 11.0),
            accent,
        );

        let canopy_x = center.x + facing * (6.0 * (1.0 - width_scale));
        draw_ellipse(
            canopy_x,
            center.y - 10.0,
            (8.0 * width_scale).max(2.0),
            8.0,
            0.0,
            canopy,
        );

        let near_wing = mix_color(accent, WHITE, 0.18);
        let far_wing = mix_color(accent, BLACK, 0.2);
        let (left_wing_color, right_wing_color) = if facing > 0.0 {
            (far_wing, near_wing)
        } else {
            (near_wing, far_wing)
        };
        draw_triangle(
            vec2(center.x - body_half * 0.45, center.y - 4.0),
            vec2(center.x - wing_span, center.y + 2.0 - wing_depth),
            vec2(center.x - body_half * 0.7, center.y + 16.0),
            left_wing_color,
        );
        draw_triangle(
            vec2(center.x + body_half * 0.45, center.y - 4.0),
            vec2(center.x + wing_span, center.y + 2.0 + wing_depth),
            vec2(center.x + body_half * 0.7, center.y + 16.0),
            right_wing_color,
        );

        let leg_x = 8.0 * width_scale;
        draw_line(
            center.x - leg_x,
            center.y + 10.0,
            center.x - leg_x,
            center.y + 24.0,
            4.0,
            color,
        );
        draw_line(
            center.x - leg_x,
            center.y + 24.0,
            center.x - leg_x - 7.0 * facing,
            center.y + 33.0,
            3.0,
            canopy,
        );
        draw_line(
            center.x + leg_x,
            center.y + 10.0,
            center.x + leg_x,
            center.y + 24.0,
            4.0,
            color,
        );
        draw_line(
            center.x + leg_x,
            center.y + 24.0,
            center.x + leg_x - 7.0 * facing,
            center.y + 33.0,
            3.0,
            canopy,
        );
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
        self.formation_x = formation_start_x();
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
        let target_x = (self.player.x + rand::gen_range(-180.0, 180.0))
            .clamp(60.0, config::WINDOW_WIDTH - 60.0);
        self.aliens[index].diving = true;
        self.aliens[index].dive_pos = center;
        self.aliens[index].dive_target_x = target_x;
        self.aliens[index].dive_vel = vec2(
            (target_x - center.x) * 0.7,
            config::ALIEN_DIVE_SPEED + self.wave as f32 * 12.0,
        );
        self.aliens[index].dive_angle = 0.0;
        self.aliens[index].fire_flash = 1.0;
        self.screen_shake = self.screen_shake.max(5.0);
        self.spawn_radial_burst(center, 10, alien_color(self.aliens[index].row), 120.0, 0.45);
    }

    fn update_diving_aliens(&mut self, dt: f32) {
        let player_rect = self.player_rect();
        let mut player_hit = false;
        let mut bunker_crashes = Vec::new();
        for (index, alien) in self.aliens.iter_mut().enumerate() {
            if !alien.alive || !alien.diving {
                continue;
            }
            let steer = (alien.dive_target_x - alien.dive_pos.x) * config::ALIEN_DIVE_TURN_RATE;
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
            if dive_rect.overlaps(&player_rect) {
                player_hit = true;
                break;
            }
            let mut bunker_hit = false;
            let mut bunker_impact = None;
            for bunker in &mut self.bunkers {
                if bunker_impact.is_none() {
                    bunker_impact = bunker.impact_point(dive_rect.center());
                }
                bunker_hit = bunker.damage_at_rect(dive_rect, true);
                if bunker_hit {
                    break;
                }
            }
            if bunker_hit {
                bunker_crashes.push((
                    index,
                    bunker_impact.unwrap_or(alien.dive_pos),
                    alien.dive_pos,
                    alien.row,
                ));
            }
        }

        if player_hit {
            self.trigger_game_over();
            return;
        }

        for (index, impact, center, row) in bunker_crashes {
            if self.aliens[index].alive && self.aliens[index].diving {
                self.aliens[index].alive = false;
                self.aliens[index].diving = false;
                self.screen_shake = self.screen_shake.max(10.0);
                self.spawn_bunker_debris(impact, config::BUNKER_COLOR);
                self.spawn_radial_burst(center, 18, alien_color(row), 190.0, 0.7);
                self.spawn_radial_burst(center, 12, config::BUNKER_COLOR, 150.0, 0.55);
            }
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
                alien.dive_target_x = 0.0;
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
        self.screen_shake = self.screen_shake.max(14.0);
        self.spawn_radial_burst(center, 30, color, 280.0, 1.0);
        self.spawn_radial_burst(center, 14, config::ACCENT_C, 180.0, 0.78);
        self.spawn_radial_burst(center, 14, WHITE, 145.0, 0.42);
        self.blast_waves.push(BlastWave {
            center,
            radius: 6.0,
            hit_player: true,
        });
    }

    fn spawn_impact(&mut self, center: Vec2, color: Color) {
        self.spawn_radial_burst(center, 15, color, 165.0, 0.48);
        self.spawn_radial_burst(center, 8, WHITE, 118.0, 0.28);
    }

    fn spawn_bunker_debris(&mut self, center: Vec2, impact_color: Color) {
        self.spawn_radial_burst(center, 9, config::BUNKER_COLOR, 118.0, 0.52);
        self.spawn_radial_burst(
            center,
            5,
            mix_color(config::BUNKER_COLOR, impact_color, 0.45),
            92.0,
            0.38,
        );
        self.spawn_radial_burst(center, 4, Color::from_rgba(38, 30, 24, 255), 72.0, 0.42);
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
        self.screen_shake = self.screen_shake.max(16.0);
        self.play_player_hit_sound();
        self.spawn_radial_burst(
            vec2(
                self.player.x,
                config::WINDOW_HEIGHT - config::PLAYER_Y_OFFSET,
            ),
            26,
            config::ACCENT_B,
            250.0,
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
            (config::ENEMY_FIRE_BASE_INTERVAL - tension * 0.78 - self.wave as f32 * 0.05)
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

    fn viewport_frame(&self) -> (Vec2, f32) {
        let scale = (screen_width() / config::WINDOW_WIDTH)
            .min(screen_height() / config::WINDOW_HEIGHT)
            .max(0.1);
        let size = vec2(config::WINDOW_WIDTH * scale, config::WINDOW_HEIGHT * scale);
        let origin = vec2(
            (screen_width() - size.x) * 0.5,
            (screen_height() - size.y) * 0.5,
        );
        (origin, scale)
    }

    fn draw_letterbox_backdrop(&self, viewport: Vec2, scale: f32) {
        let view_w = config::WINDOW_WIDTH * scale;
        let view_h = config::WINDOW_HEIGHT * scale;
        draw_glow_circle(
            screen_width() * 0.5,
            screen_height() * 0.5,
            screen_width().max(screen_height()) * 0.45,
            Color::from_rgba(44, 160, 255, 28),
        );
        draw_glow_circle(
            screen_width() * 0.5,
            screen_height() * 0.34,
            screen_width().max(screen_height()) * 0.32,
            Color::from_rgba(255, 96, 154, 18),
        );
        draw_rectangle(
            viewport.x - 24.0,
            viewport.y - 24.0,
            view_w + 48.0,
            view_h + 48.0,
            Color::from_rgba(5, 8, 22, 120),
        );
    }

    fn draw_viewport_frame(&self, viewport: Vec2, scale: f32) {
        let view_w = config::WINDOW_WIDTH * scale;
        let view_h = config::WINDOW_HEIGHT * scale;
        draw_rectangle_lines(
            viewport.x - 2.0,
            viewport.y - 2.0,
            view_w + 4.0,
            view_h + 4.0,
            2.0,
            Color::from_rgba(255, 255, 255, 24),
        );
        draw_rectangle_lines(
            viewport.x,
            viewport.y,
            view_w,
            view_h,
            2.0,
            Color::from_rgba(82, 218, 255, 96),
        );
        draw_line(
            viewport.x + 28.0,
            viewport.y + 14.0,
            viewport.x + view_w - 28.0,
            viewport.y + 14.0,
            1.0,
            Color::from_rgba(255, 255, 255, 18),
        );
    }
}

impl Bunker {
    fn impact_point(&self, point: Vec2) -> Option<Vec2> {
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
                if cell_rect.contains(point) {
                    return Some(cell_rect.center());
                }
            }
        }
        None
    }

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
                let damage = 3 - hp;
                let inset = damage as f32 * 0.9;
                let x = self.origin.x + col as f32 * config::BUNKER_CELL + offset.x + inset * 0.4;
                let y = self.origin.y + row as f32 * config::BUNKER_CELL + offset.y + inset * 0.25;
                let cell_w = (config::BUNKER_CELL - 1.5 - inset).max(6.0);
                let cell_h = (config::BUNKER_CELL - 1.5 - inset * 0.9).max(6.0);
                let base = match hp {
                    3 => config::BUNKER_COLOR,
                    2 => Color::from_rgba(238, 225, 108, 255),
                    _ => Color::from_rgba(255, 112, 94, 255),
                };
                let bevel = mix_color(base, WHITE, 0.22);
                let shadow = mix_color(base, BLACK, 0.32);
                let side_facet = mix_color(base, BLACK, 0.16);
                let top_facet = mix_color(base, WHITE, 0.34);
                draw_rectangle(x, y, cell_w, cell_h, base);
                draw_rectangle(
                    x + 2.0,
                    y + 5.0,
                    (cell_w - 3.0).max(3.0),
                    (cell_h - 6.0).max(3.0),
                    Color::new(shadow.r, shadow.g, shadow.b, 0.85),
                );
                draw_rectangle(
                    x + cell_w - 5.0,
                    y + 2.0,
                    4.0,
                    (cell_h - 6.0).max(2.0),
                    Color::new(side_facet.r, side_facet.g, side_facet.b, 0.95),
                );
                draw_rectangle(
                    x + 1.0,
                    y + 1.0,
                    (cell_w - 3.0).max(2.0),
                    3.0,
                    Color::new(bevel.r, bevel.g, bevel.b, 0.82),
                );
                draw_rectangle(
                    x + 2.0,
                    y + 2.0,
                    (cell_w - 6.0).max(1.0),
                    2.0,
                    Color::new(top_facet.r, top_facet.g, top_facet.b, 0.9),
                );
                draw_rectangle(
                    x - 1.0,
                    y - 1.0,
                    cell_w + 1.5,
                    cell_h + 1.5,
                    Color::new(base.r, base.g, base.b, 0.12),
                );
                if hp == 3 {
                    draw_glow_circle(
                        x + cell_w * 0.5,
                        y + cell_h * 0.45,
                        8.0,
                        Color::new(base.r, base.g, base.b, 0.08),
                    );
                }
                if hp <= 2 {
                    draw_line(
                        x + 3.0,
                        y + 3.0,
                        x + cell_w - 3.0,
                        y + cell_h - 4.0,
                        1.6,
                        Color::from_rgba(24, 16, 12, 160),
                    );
                    draw_line(
                        x + cell_w - 5.0,
                        y + 4.0,
                        x + 5.0,
                        y + cell_h - 3.0,
                        1.2,
                        Color::from_rgba(255, 255, 255, 44),
                    );
                    draw_circle(
                        x + cell_w * 0.74,
                        y + cell_h * 0.3,
                        1.6,
                        Color::from_rgba(28, 18, 16, 180),
                    );
                }
                if hp == 1 {
                    draw_circle(
                        x + cell_w * 0.55,
                        y + cell_h * 0.52,
                        3.4,
                        Color::from_rgba(18, 10, 8, 210),
                    );
                    draw_circle(
                        x + cell_w * 0.22,
                        y + cell_h * 0.72,
                        2.2,
                        Color::from_rgba(24, 12, 10, 170),
                    );
                    draw_rectangle(
                        x + cell_w - 4.0,
                        y + 1.5,
                        2.5,
                        4.0,
                        Color::from_rgba(14, 8, 6, 170),
                    );
                }
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
                dive_target_x: 0.0,
                dive_angle: 0.0,
            });
        }
    }
    aliens
}

fn build_bunkers() -> Vec<Bunker> {
    let mut bunkers = Vec::with_capacity(config::BUNKER_COUNT);
    let bunker_width = config::BUNKER_GRID_W as f32 * config::BUNKER_CELL;
    let group_width = bunker_width * config::BUNKER_COUNT as f32;
    let gap =
        ((config::WINDOW_WIDTH - group_width) / (config::BUNKER_COUNT as f32 + 1.0)).max(82.0);
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
            origin: vec2(gap + i as f32 * (bunker_width + gap), config::BUNKER_Y),
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
                draw_line(
                    pos.x,
                    pos.y - trail_h,
                    pos.x,
                    pos.y + shot.size.y * 0.4,
                    3.0,
                    Color::new(color.r, color.g, color.b, 0.18 + flicker * 0.1),
                );
                draw_rectangle(
                    pos.x - shot.size.x * 0.9,
                    pos.y - trail_h * 0.68,
                    shot.size.x * 1.8,
                    trail_h * 0.62,
                    Color::from_rgba(255, 220, 150, ((70.0 + flicker * 60.0).round()) as u8),
                );
            }
            draw_glow_circle(
                pos.x,
                pos.y,
                shot.size.x * if shot.from_player { 2.0 } else { 2.35 },
                Color::new(
                    color.r,
                    color.g,
                    color.b,
                    if shot.from_player { 0.10 } else { 0.14 },
                ),
            );
            draw_rectangle(
                pos.x - shot.size.x * 1.2,
                pos.y - shot.size.y * 0.5,
                shot.size.x * 2.4,
                shot.size.y,
                glow,
            );
            draw_circle(
                pos.x,
                pos.y,
                shot.size.x * if shot.from_player { 0.9 } else { 1.1 },
                Color::new(
                    core.r,
                    core.g,
                    core.b,
                    if shot.from_player { 0.2 } else { 0.28 },
                ),
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
            draw_glow_circle(
                pos.x,
                pos.y,
                shot.size.x * (1.65 + pulse * 0.2),
                Color::from_rgba(255, 82, 72, (34.0 + pulse * 22.0).round() as u8),
            );
            draw_glow_circle(
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
            let pulse = (get_time() as f32 * 14.0 + shot.pos.y * 0.05).sin() * 0.5 + 0.5;
            draw_glow_circle(
                pos.x,
                pos.y,
                shot.size.x * (1.3 + pulse * 0.16),
                Color::from_rgba(255, 214, 92, (38.0 + pulse * 24.0).round() as u8),
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
            draw_line(
                pos.x,
                pos.y + shot.size.y * 0.3,
                pos.x,
                pos.y + shot.size.y * 1.9,
                6.0,
                Color::from_rgba(100, 244, 255, (40.0 + pulse * 28.0).round() as u8),
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

fn formation_start_x() -> f32 {
    let formation_width = (config::ALIEN_COLS - 1) as f32 * config::ALIEN_SPACING_X;
    config::ALIEN_START_X
        + (config::WINDOW_WIDTH * 0.5 - (config::ALIEN_START_X + formation_width * 0.5))
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

fn draw_glow_circle(x: f32, y: f32, radius: f32, color: Color) {
    draw_circle(
        x,
        y,
        radius,
        Color::new(color.r, color.g, color.b, color.a * 0.45),
    );
    draw_circle(
        x,
        y,
        radius * 0.6,
        Color::new(color.r, color.g, color.b, color.a * 0.28),
    );
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

fn draw_hud_card(x: f32, y: f32, w: f32, h: f32, color: Color) {
    draw_rectangle(x, y, w, h, config::HUD_PANEL_BG);
    draw_rectangle(
        x + 6.0,
        y + 6.0,
        w - 12.0,
        14.0,
        Color::new(
            config::HUD_PANEL_INNER.r,
            config::HUD_PANEL_INNER.g,
            config::HUD_PANEL_INNER.b,
            0.65,
        ),
    );
    draw_rectangle(
        x + 10.0,
        y + h - 14.0,
        w - 20.0,
        4.0,
        Color::new(color.r, color.g, color.b, 0.18),
    );
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(255, 255, 255, 24));
    draw_rectangle_lines(
        x + 4.0,
        y + 4.0,
        w - 8.0,
        h - 8.0,
        2.0,
        Color::new(color.r, color.g, color.b, 0.78),
    );
    draw_line(
        x + 10.0,
        y + 10.0,
        x + w - 10.0,
        y + 10.0,
        1.0,
        Color::new(
            config::HUD_PANEL_EDGE.r,
            config::HUD_PANEL_EDGE.g,
            config::HUD_PANEL_EDGE.b,
            0.3,
        ),
    );
}

fn draw_holo_frame(x: f32, y: f32, w: f32, h: f32, color: Color) {
    draw_glow_circle(
        x + w * 0.5,
        y + h * 0.5,
        w.max(h) * 0.42,
        Color::new(color.r, color.g, color.b, 0.04),
    );
    draw_hud_card(x, y, w, h, color);
    draw_line(x + 26.0, y, x + 120.0, y, 3.0, color);
    draw_line(x, y + 24.0, x, y + 110.0, 3.0, color);
    draw_line(x + w - 120.0, y + h, x + w - 26.0, y + h, 3.0, color);
    draw_line(x + w, y + h - 110.0, x + w, y + h - 24.0, 3.0, color);
}

fn profiles_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
        return Some(base.join("Neon Invaders").join("profiles.txt"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })?;
        Some(base.join("neon-invaders").join("profiles.txt"))
    }
}

fn load_profiles() -> Vec<PlayerProfile> {
    let Some(path) = profiles_path() else {
        return Vec::new();
    };
    let mut profiles = Vec::new();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.lines() {
            let mut parts = line.splitn(2, '\t');
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(score_text) = parts.next() else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let Ok(high_score) = score_text.trim().parse::<u32>() else {
                continue;
            };
            profiles.push(PlayerProfile {
                name: name.to_string(),
                high_score,
            });
        }
    }
    profiles.sort_by(|a, b| b.high_score.cmp(&a.high_score).then(a.name.cmp(&b.name)));
    profiles
}

fn save_profiles(profiles: &[PlayerProfile]) {
    let Some(path) = profiles_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut text = String::new();
    for profile in profiles {
        text.push_str(&profile.name.replace(['\n', '\t'], " "));
        text.push('\t');
        text.push_str(&profile.high_score.to_string());
        text.push('\n');
    }
    let _ = fs::write(path, text);
}
