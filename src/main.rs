mod audio;
mod config;
mod game;

use game::GameApp;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Neon Invaders".to_owned(),
        window_width: config::WINDOW_WIDTH as i32,
        window_height: config::WINDOW_HEIGHT as i32,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = GameApp::new().await;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);
        app.update(dt);
        app.draw();
        next_frame().await;
    }
}
