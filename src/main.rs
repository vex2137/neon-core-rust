use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;

const ARENA_RADIUS: f32 = 1350.0;
const DEMO_MAX_WAVE: u32 = 8;

#[derive(Clone, Copy, PartialEq)]
enum ScreenState {
    Menu,
    Playing,
    Paused,
    Settings,
    LevelUp,
    GameOver { victory: bool },
}

#[derive(Clone, Copy)]
struct Settings {
    sound: bool,
    shake: bool,
    dmg_nums: bool,
    fixed_stick: bool,
    auto_aim: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            shake: true,
            dmg_nums: true,
            fixed_stick: false,
            auto_aim: true,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct MetaSave {
    shards: u32,
    dmg_lvl: u32,
    rate_lvl: u32,
    hp_lvl: u32,
    spd_lvl: u32,
}

impl MetaSave {
    fn save_path() -> &'static str {
        if std::path::Path::new("/data/data/dev.vex.neoncore/files").exists() {
            "/data/data/dev.vex.neoncore/files/save.dat"
        } else {
            "neon_core_save.dat"
        }
    }

    fn load() -> Self {
        let mut s = Self::default();
        if let Ok(data) = std::fs::read_to_string(Self::save_path()) {
            let parts: Vec<u32> = data
                .split(',')
                .filter_map(|p| p.trim().parse().ok())
                .collect();
            if parts.len() == 5 {
                s.shards = parts[0];
                s.dmg_lvl = parts[1];
                s.rate_lvl = parts[2];
                s.hp_lvl = parts[3];
                s.spd_lvl = parts[4];
            }
        }
        s
    }

    fn save(&self) {
        let content = format!(
            "{},{},{},{},{}",
            self.shards, self.dmg_lvl, self.rate_lvl, self.hp_lvl, self.spd_lvl
        );
        let _ = std::fs::write(Self::save_path(), content);
    }
}

#[derive(Clone, Copy)]
struct Bullet {
    active: bool,
    pos: Vec2,
    vel: Vec2,
    dmg: f32,
    life: f32,
    pierce: i32,
}

#[derive(Clone, Copy)]
struct Enemy {
    active: bool,
    kind: u8, // 0: Swarmer, 1: Dasher, 2: Tank, 3: Splitter Boss
    pos: Vec2,
    hp: f32,
    max_hp: f32,
    radius: f32,
    speed: f32,
    flash: f32,
    timer: f32,
    angle: f32,
    spin: f32,
}

#[derive(Clone, Copy)]
struct Particle {
    active: bool,
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

#[derive(Clone, Copy)]
struct Ring {
    active: bool,
    pos: Vec2,
    radius: f32,
    max_radius: f32,
    life: f32,
    max_life: f32,
    color: Color,
}

#[derive(Clone)]
struct DmgText {
    active: bool,
    pos: Vec2,
    val: String,
    life: f32,
    color: Color,
}

#[derive(Clone, Copy)]
struct Gem {
    active: bool,
    pos: Vec2,
    xp: f32,
    shard: u32,
}

#[derive(Clone, Copy)]
struct ArcBolt {
    active: bool,
    a: Vec2,
    b: Vec2,
    life: f32,
}

#[derive(Clone, Copy)]
struct UpgradeCard {
    id: u8,
    title: &'static str,
    desc: &'static str,
}

const ALL_UPGRADES: [UpgradeCard; 10] = [
    UpgradeCard { id: 0, title: "ROZDZIELACZ LUF", desc: "+1 Pocisk na salwe" },
    UpgradeCard { id: 1, title: "PLAZMA OVERCHARGE", desc: "+25% Obrazen broni" },
    UpgradeCard { id: 2, title: "CEWKI CYKLERA", desc: "+20% Szybkostrzelnosci" },
    UpgradeCard { id: 3, title: "PENETRATOR", desc: "+1 Przebicie wrogow" },
    UpgradeCard { id: 4, title: "PILA ORBITALNA", desc: "+1 Wirujace ostrze" },
    UpgradeCard { id: 5, title: "LUK TESLI", desc: "Blyskawice lancuchowe" },
    UpgradeCard { id: 6, title: "IMPULS EMP", desc: "Radialna fala uderzeniowa" },
    UpgradeCard { id: 7, title: "CHLODZENIE DASH", desc: "-25% Cooldownu uniku" },
    UpgradeCard { id: 8, title: "NANONAPRAWA", desc: "+40% HP i +25 Max HP" },
    UpgradeCard { id: 9, title: "MAGNES RDZENIA", desc: "+45% Zasieg zbierania" },
];

struct SoundBank {
    shoot: Option<Sound>,
    hit: Option<Sound>,
    boom: Option<Sound>,
    dash: Option<Sound>,
    lvlup: Option<Sound>,
    shoot_cd: f32,
}

fn synth_wav(freq_start: f32, freq_end: f32, duration: f32, volume: f32, square: bool) -> Vec<u8> {
    let sample_rate = 22050u32;
    let num_samples = (sample_rate as f32 * duration) as usize;
    let data_size = (num_samples * 2) as u32;
    let mut buf = Vec::with_capacity(44 + data_size as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVEfmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // Mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    let mut phase = 0.0f32;
    for i in 0..num_samples {
        let t = i as f32 / num_samples as f32;
        let freq = freq_start * (freq_end / freq_start).powf(t);
        phase += freq / sample_rate as f32;
        let wave = if square {
            if phase.fract() < 0.5 { 1.0 } else { -1.0 }
        } else {
            (phase * std::f32::consts::TAU).sin()
        };
        let env = (1.0 - t).powf(1.6) * volume;
        let sample = (wave * env * 32767.0).clamp(-32767.0, 32767.0) as i16;
        buf.extend_from_slice(&sample.to_le_bytes());
    }
    buf
}

impl SoundBank {
    async fn new() -> Self {
        Self {
            shoot: load_sound_from_bytes(&synth_wav(460.0, 110.0, 0.07, 0.22, true)).await.ok(),
            hit: load_sound_from_bytes(&synth_wav(190.0, 55.0, 0.05, 0.25, false)).await.ok(),
            boom: load_sound_from_bytes(&synth_wav(135.0, 24.0, 0.24, 0.42, true)).await.ok(),
            dash: load_sound_from_bytes(&synth_wav(170.0, 540.0, 0.14, 0.30, false)).await.ok(),
            lvlup: load_sound_from_bytes(&synth_wav(330.0, 880.0, 0.28, 0.32, false)).await.ok(),
            shoot_cd: 0.0,
        }
    }

    fn play(&self, snd: &Option<Sound>, enabled: bool, vol: f32) {
        if !enabled {
            return;
        }
        if let Some(s) = snd {
            play_sound(
                s,
                PlaySoundParams {
                    looped: false,
                    volume: vol,
                },
            );
        }
    }
}

struct Stick {
    touch_id: Option<u64>,
    start: Vec2,
    cur: Vec2,
    dir: Vec2,
}

impl Stick {
    fn new() -> Self {
        Self {
            touch_id: None,
            start: Vec2::ZERO,
            cur: Vec2::ZERO,
            dir: Vec2::ZERO,
        }
    }
}

struct Game {
    state: ScreenState,
    prev_state: ScreenState,
    settings: Settings,
    meta: MetaSave,
    sounds: SoundBank,

    bullets: Vec<Bullet>,
    enemies: Vec<Enemy>,
    particles: Vec<Particle>,
    rings: Vec<Ring>,
    texts: Vec<DmgText>,
    gems: Vec<Gem>,
    arcs: Vec<ArcBolt>,
    stars: Vec<(Vec2, f32, f32)>,

    player_pos: Vec2,
    player_vel: Vec2,
    aim_angle: f32,
    thrust_accum: f32,

    dash_timer: f32,
    dash_cd: f32,
    dash_max_cd: f32,
    dash_dir: Vec2,

    hp: f32,
    max_hp: f32,
    xp: f32,
    next_xp: f32,
    level: u32,
    wave: u32,
    wave_timer: f32,
    spawn_accum: f32,
    run_shards: u32,
    kills: u32,
    shake: f32,

    move_speed: f32,
    fire_cd: f32,
    fire_interval: f32,
    bullet_dmg: f32,
    bullet_count: i32,
    bullet_pierce: i32,
    magnet_radius: f32,

    orbit_count: i32,
    orbit_angle: f32,
    orbit_dmg: f32,
    tesla_lvl: i32,
    tesla_timer: f32,
    nova_lvl: i32,
    nova_timer: f32,

    left_stick: Stick,
    right_stick: Stick,
    level_choices: [UpgradeCard; 3],
}

impl Game {
    fn new(sounds: SoundBank) -> Self {
        let stars = (0..140)
            .map(|_| {
                (
                    vec2(rand::gen_range(-1600.0, 1600.0), rand::gen_range(-1600.0, 1600.0)),
                    if rand::gen_range(0, 10) < 6 { 0.25 } else { 0.55 },
                    rand::gen_range(1.5, 2.6),
                )
            })
            .collect();

        Self {
            state: ScreenState::Menu,
            prev_state: ScreenState::Menu,
            settings: Settings::default(),
            meta: MetaSave::load(),
            sounds,

            bullets: vec![
                Bullet { active: false, pos: Vec2::ZERO, vel: Vec2::ZERO, dmg: 0.0, life: 0.0, pierce: 0 };
                400
            ],
            enemies: vec![
                Enemy {
                    active: false, kind: 0, pos: Vec2::ZERO, hp: 0.0, max_hp: 0.0,
                    radius: 12.0, speed: 100.0, flash: 0.0, timer: 0.0, angle: 0.0, spin: 0.0
                };
                320
            ],
            particles: vec![
                Particle {
                    active: false, pos: Vec2::ZERO, vel: Vec2::ZERO, life: 0.0,
                    max_life: 1.0, size: 3.0, color: SKYBLUE
                };
                550
            ],
            rings: vec![
                Ring { active: false, pos: Vec2::ZERO, radius: 0.0, max_radius: 100.0, life: 0.0, max_life: 0.3, color: SKYBLUE };
                32
            ],
            texts: vec![
                DmgText { active: false, pos: Vec2::ZERO, val: String::new(), life: 0.0, color: WHITE };
                75
            ],
            gems: vec![Gem { active: false, pos: Vec2::ZERO, xp: 0.0, shard: 0 }; 300],
            arcs: vec![ArcBolt { active: false, a: Vec2::ZERO, b: Vec2::ZERO, life: 0.0 }; 48],
            stars,

            player_pos: Vec2::ZERO,
            player_vel: Vec2::ZERO,
            aim_angle: 0.0,
            thrust_accum: 0.0,

            dash_timer: 0.0,
            dash_cd: 0.0,
            dash_max_cd: 1.6,
            dash_dir: Vec2::X,

            hp: 100.0,
            max_hp: 100.0,
            xp: 0.0,
            next_xp: 25.0,
            level: 1,
            wave: 1,
            wave_timer: 0.0,
            spawn_accum: 0.0,
            run_shards: 0,
            kills: 0,
            shake: 0.0,

            move_speed: 275.0,
            fire_cd: 0.0,
            fire_interval: 0.21,
            bullet_dmg: 18.0,
            bullet_count: 1,
            bullet_pierce: 1,
            magnet_radius: 130.0,

            orbit_count: 0,
            orbit_angle: 0.0,
            orbit_dmg: 16.0,
            tesla_lvl: 0,
            tesla_timer: 0.0,
            nova_lvl: 0,
            nova_timer: 0.0,

            left_stick: Stick::new(),
            right_stick: Stick::new(),
            level_choices: [ALL_UPGRADES[0], ALL_UPGRADES[1], ALL_UPGRADES[2]],
        }
    }

    fn start_run(&mut self) {
        for b in &mut self.bullets { b.active = false; }
        for e in &mut self.enemies { e.active = false; }
        for p in &mut self.particles { p.active = false; }
        for r in &mut self.rings { r.active = false; }
        for t in &mut self.texts { t.active = false; }
        for g in &mut self.gems { g.active = false; }
        for a in &mut self.arcs { a.active = false; }

        self.left_stick = Stick::new();
        self.right_stick = Stick::new();

        self.player_pos = Vec2::ZERO;
        self.player_vel = Vec2::ZERO;
        self.dash_timer = 0.0;
        self.dash_cd = 0.0;
        self.dash_max_cd = 1.6;

        self.wave = 1;
        self.wave_timer = 0.0;
        self.spawn_accum = 0.0;
        self.level = 1;
        self.xp = 0.0;
        self.next_xp = 25.0;
        self.run_shards = 0;
        self.kills = 0;

        self.max_hp = 100.0 + self.meta.hp_lvl as f32 * 25.0;
        self.hp = self.max_hp;
        self.bullet_dmg = 18.0 * (1.0 + self.meta.dmg_lvl as f32 * 0.15);
        self.fire_interval = (0.21 * 0.92f32.powi(self.meta.rate_lvl as i32)).max(0.06);
        self.move_speed = 275.0 * (1.0 + self.meta.spd_lvl as f32 * 0.08);
        self.bullet_count = 1;
        self.bullet_pierce = 1;
        self.magnet_radius = 130.0;

        self.orbit_count = 0;
        self.orbit_dmg = 16.0;
        self.tesla_lvl = 0;
        self.nova_lvl = 0;

        for _ in 0..6 {
            self.spawn_enemy(0);
        }
        self.state = ScreenState::Playing;
    }

    fn trigger_dash(&mut self) {
        if self.dash_cd > 0.0 {
            return;
        }
        self.sounds.play(&self.sounds.dash, self.settings.sound, 0.8);
        let mut dir = self.left_stick.dir;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { dir.y -= 1.0; }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { dir.y += 1.0; }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) { dir.x -= 1.0; }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { dir.x += 1.0; }

        if dir.length_squared() > 0.01 {
            self.dash_dir = dir.normalize();
        } else {
            self.dash_dir = Vec2::from_angle(self.aim_angle);
        }
        self.dash_timer = 0.18;
        self.dash_cd = self.dash_max_cd;
        self.spawn_ring(self.player_pos, 75.0, Color::from_rgba(5, 217, 232, 255), 0.25);
    }

    fn update_playing(&mut self, dt: f32) {
        let sw = screen_width();
        let sh = screen_height();

        let pause_rect = Rect::new(sw - 54.0, 12.0, 42.0, 42.0);
        let dash_center = vec2(sw - 58.0, sh - 62.0);

        // Touch handling
        for touch in touches() {
            match touch.phase {
                TouchPhase::Started => {
                    if pause_rect.contains(touch.position) {
                        self.state = ScreenState::Paused;
                        return;
                    }
                    if touch.position.distance(dash_center) <= 44.0 {
                        self.trigger_dash();
                        continue;
                    }
                    let want_move = if self.settings.auto_aim {
                        self.left_stick.touch_id.is_none()
                    } else {
                        touch.position.x < sw * 0.5 && self.left_stick.touch_id.is_none()
                    };

                    if want_move {
                        let start = if self.settings.fixed_stick {
                            vec2(86.0, sh - 96.0)
                        } else {
                            touch.position
                        };
                        self.left_stick.touch_id = Some(touch.id);
                        self.left_stick.start = start;
                        self.left_stick.cur = touch.position;
                        let delta = touch.position - start;
                        let len = delta.length();
                        self.left_stick.dir = if len > 2.0 {
                            (delta / len) * (len.min(44.0) / 44.0)
                        } else {
                            Vec2::ZERO
                        };
                    } else if self.right_stick.touch_id.is_none() {
                        self.right_stick.touch_id = Some(touch.id);
                        self.right_stick.start = touch.position;
                        self.right_stick.cur = touch.position;
                        self.right_stick.dir = Vec2::ZERO;
                    }
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if Some(touch.id) == self.left_stick.touch_id {
                        self.left_stick.cur = touch.position;
                        let delta = touch.position - self.left_stick.start;
                        let len = delta.length();
                        self.left_stick.dir = if len > 2.0 {
                            (delta / len) * (len.min(44.0) / 44.0)
                        } else {
                            Vec2::ZERO
                        };
                    } else if Some(touch.id) == self.right_stick.touch_id {
                        self.right_stick.cur = touch.position;
                        let delta = touch.position - self.right_stick.start;
                        let len = delta.length();
                        self.right_stick.dir = if len > 4.0 { delta / len } else { Vec2::ZERO };
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if Some(touch.id) == self.left_stick.touch_id {
                        self.left_stick.touch_id = None;
                        self.left_stick.dir = Vec2::ZERO;
                    } else if Some(touch.id) == self.right_stick.touch_id {
                        self.right_stick.touch_id = None;
                        self.right_stick.dir = Vec2::ZERO;
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::LeftShift) {
            self.trigger_dash();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.state = ScreenState::Paused;
            return;
        }

        // Wave timer
        self.wave_timer += dt;
        if self.wave_timer >= 20.0 {
            self.wave_timer = 0.0;
            self.wave += 1;
            if cfg!(feature = "demo") && self.wave > DEMO_MAX_WAVE {
                self.finish_run(true);
                return;
            }
            if self.wave % 3 == 0 {
                self.spawn_enemy(3);
            }
        }

        // Spawner
        let spawn_rate = (1.6 + self.wave as f32 * 0.55).min(9.5);
        self.spawn_accum += dt * spawn_rate;
        while self.spawn_accum >= 1.0 {
            self.spawn_accum -= 1.0;
            let roll = rand::gen_range(0.0f32, 1.0);
            let kind = if self.wave >= 3 && roll > 0.90 {
                2
            } else if self.wave >= 2 && roll > 0.72 {
                1
            } else {
                0
            };
            self.spawn_enemy(kind);
        }

        // Player physics
        if self.dash_cd > 0.0 {
            self.dash_cd = (self.dash_cd - dt).max(0.0);
        }

        if self.dash_timer > 0.0 {
            self.dash_timer -= dt;
            self.player_vel = self.dash_dir * 820.0;
            self.spawn_particle(
                self.player_pos,
                vec2(rand::gen_range(-45.0, 45.0), rand::gen_range(-45.0, 45.0)),
                0.22,
                5.0,
                Color::from_rgba(5, 217, 232, 255),
            );
        } else {
            let mut mv = self.left_stick.dir;
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { mv.y -= 1.0; }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { mv.y += 1.0; }
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) { mv.x -= 1.0; }
            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { mv.x += 1.0; }
            if mv.length_squared() > 1.0 {
                mv = mv.normalize();
            }

            let target_vel = mv * self.move_speed;
            let accel = 1.0 - (-18.0 * dt).exp();
            self.player_vel += (target_vel - self.player_vel) * accel;

            if self.player_vel.length() > 35.0 {
                self.thrust_accum += dt * 45.0;
                while self.thrust_accum >= 1.0 {
                    self.thrust_accum -= 1.0;
                    let back = self.player_vel.y.atan2(self.player_vel.x)
                        + std::f32::consts::PI
                        + rand::gen_range(-0.25, 0.25);
                    let dir = Vec2::from_angle(back);
                    self.spawn_particle(
                        self.player_pos + dir * 12.0,
                        dir * 120.0,
                        0.2,
                        3.0,
                        Color::from_rgba(5, 217, 232, 255),
                    );
                }
            }
        }

        self.player_pos += self.player_vel * dt;
        let dist_center = self.player_pos.length();
        if dist_center > ARENA_RADIUS - 18.0 {
            self.player_pos = (self.player_pos / dist_center) * (ARENA_RADIUS - 18.0);
        }

        // Aiming
        let manual_aim = self.right_stick.touch_id.is_some() && self.right_stick.dir.length_squared() > 0.01;
        let nearest_pos = self.find_nearest_enemy(self.player_pos, 580.0);
        if manual_aim {
            self.aim_angle = self.right_stick.dir.y.atan2(self.right_stick.dir.x);
        } else if self.settings.auto_aim {
            if let Some(target) = nearest_pos {
                let d = target - self.player_pos;
                self.aim_angle = d.y.atan2(d.x);
            } else if self.player_vel.length() > 20.0 {
                self.aim_angle = self.player_vel.y.atan2(self.player_vel.x);
            }
        } else if self.player_vel.length() > 20.0 {
            self.aim_angle = self.player_vel.y.atan2(self.player_vel.x);
        }

        // Primary weapon
        self.sounds.shoot_cd = (self.sounds.shoot_cd - dt).max(0.0);
        self.fire_cd -= dt;
        if self.fire_cd <= 0.0 && (manual_aim || !self.settings.auto_aim || nearest_pos.is_some()) {
            self.fire_cd = self.fire_interval;
            if self.sounds.shoot_cd <= 0.0 {
                self.sounds.play(&self.sounds.shoot, self.settings.sound, 0.5);
                self.sounds.shoot_cd = 0.055;
            }
            let spread = 0.11;
            for i in 0..self.bullet_count {
                let offset = (i as f32 - (self.bullet_count - 1) as f32 * 0.5) * spread;
                let a = self.aim_angle + offset;
                let dir = Vec2::from_angle(a);
                self.spawn_bullet(self.player_pos + dir * 18.0, dir * 720.0, self.bullet_dmg, self.bullet_pierce);
            }
        }

        // Orbital saws
        if self.orbit_count > 0 {
            self.orbit_angle += dt * 3.6;
            for i in 0..self.orbit_count {
                let a = self.orbit_angle + (i as f32 * std::f32::consts::TAU) / self.orbit_count as f32;
                let bpos = self.player_pos + Vec2::from_angle(a) * 72.0;
                for j in 0..self.enemies.len() {
                    if !self.enemies[j].active { continue; }
                    if self.enemies[j].pos.distance(bpos) < self.enemies[j].radius + 12.0 {
                        self.damage_enemy(j, self.orbit_dmg * dt * 8.0, bpos, false);
                    }
                }
            }
        }

        // Chain Tesla
        if self.tesla_lvl > 0 {
            self.tesla_timer -= dt;
            if self.tesla_timer <= 0.0 {
                self.tesla_timer = (1.4 - self.tesla_lvl as f32 * 0.18).max(0.45);
                self.trigger_tesla();
            }
        }

        // EMP Nova
        if self.nova_lvl > 0 {
            self.nova_timer -= dt;
            if self.nova_timer <= 0.0 {
                self.nova_timer = (4.5 - self.nova_lvl as f32 * 0.45).max(1.8);
                self.trigger_nova();
            }
        }

        // Update bullets
        for i in 0..self.bullets.len() {
            if !self.bullets[i].active { continue; }
            self.bullets[i].pos += self.bullets[i].vel * dt;
            self.bullets[i].life -= dt;
            if self.bullets[i].life <= 0.0 || self.bullets[i].pos.length() > ARENA_RADIUS {
                self.bullets[i].active = false;
                continue;
            }
            let bpos = self.bullets[i].pos;
            let bdmg = self.bullets[i].dmg;
            for j in 0..self.enemies.len() {
                if !self.enemies[j].active { continue; }
                if self.enemies[j].pos.distance(bpos) < self.enemies[j].radius + 7.0 {
                    self.damage_enemy(j, bdmg, bpos, true);
                    self.bullets[i].pierce -= 1;
                    if self.bullets[i].pierce <= 0 {
                        self.bullets[i].active = false;
                        break;
                    }
                }
            }
        }

        // Update enemies
        for i in 0..self.enemies.len() {
            if !self.enemies[i].active { continue; }
            if self.enemies[i].flash > 0.0 {
                self.enemies[i].flash -= dt;
            }
            let to_player = self.player_pos - self.enemies[i].pos;
            let dist = to_player.length().max(0.001);
            let dir = to_player / dist;
            self.enemies[i].angle += self.enemies[i].spin * dt;

            let mult = if self.enemies[i].kind == 1 {
                self.enemies[i].timer += dt;
                if (self.enemies[i].timer % 2.2) > 1.65 { 3.1 } else { 0.65 }
            } else {
                1.0
            };
            let spd = self.enemies[i].speed * mult;
            self.enemies[i].pos += dir * spd * dt;

            if self.dash_timer <= 0.0 && dist < self.enemies[i].radius + 14.0 {
                let dps = if self.enemies[i].kind == 3 { 70.0 } else { 40.0 };
                self.hp -= dps * dt;
                if self.settings.shake {
                    self.shake = self.shake.max(5.0);
                }
                if self.hp <= 0.0 {
                    self.hp = 0.0;
                    self.finish_run(false);
                    return;
                }
            }
        }

        // Update gems
        for i in 0..self.gems.len() {
            if !self.gems[i].active { continue; }
            let to_p = self.player_pos - self.gems[i].pos;
            let dist = to_p.length();
            if dist < self.magnet_radius && dist > 0.1 {
                self.gems[i].pos += (to_p / dist) * 490.0 * dt;
            }
            if dist < 24.0 {
                self.gems[i].active = false;
                self.xp += self.gems[i].xp;
                self.run_shards += self.gems[i].shard;
                if self.xp >= self.next_xp {
                    self.xp -= self.next_xp;
                    self.level += 1;
                    self.next_xp = (self.next_xp * 1.32 + 12.0).floor();
                    self.sounds.play(&self.sounds.lvlup, self.settings.sound, 0.8);
                    self.spawn_ring(self.player_pos, 150.0, Color::from_rgba(125, 249, 255, 255), 0.4);
                    self.roll_levelup();
                    return;
                }
            }
        }

        // Update particles, rings, texts, arcs
        for p in &mut self.particles {
            if !p.active { continue; }
            p.pos += p.vel * dt;
            p.life -= dt;
            if p.life <= 0.0 { p.active = false; }
        }
        for r in &mut self.rings {
            if !r.active { continue; }
            r.life -= dt;
            r.radius = r.max_radius * (1.0 - (r.life / r.max_life).max(0.0));
            if r.life <= 0.0 { r.active = false; }
        }
        for t in &mut self.texts {
            if !t.active { continue; }
            t.pos.y -= 52.0 * dt;
            t.life -= dt;
            if t.life <= 0.0 { t.active = false; }
        }
        for a in &mut self.arcs {
            if !a.active { continue; }
            a.life -= dt;
            if a.life <= 0.0 { a.active = false; }
        }

        if self.shake > 0.0 {
            self.shake = (self.shake - dt * 30.0).max(0.0);
        }
    }

    fn trigger_tesla(&mut self) {
        let mut cur = self.player_pos;
        let mut hit = Vec::with_capacity(8);
        let jumps = 2 + self.tesla_lvl;
        for _ in 0..jumps {
            let mut best: Option<usize> = None;
            let mut best_d = 250.0;
            for i in 0..self.enemies.len() {
                if !self.enemies[i].active || hit.contains(&i) { continue; }
                let d = self.enemies[i].pos.distance(cur);
                if d < best_d {
                    best_d = d;
                    best = Some(i);
                }
            }
            if let Some(idx) = best {
                hit.push(idx);
                let target_pos = self.enemies[idx].pos;
                self.spawn_arc(cur, target_pos);
                let dmg = self.bullet_dmg * (0.8 + self.tesla_lvl as f32 * 0.25);
                self.damage_enemy(idx, dmg, target_pos, true);
                cur = target_pos;
            } else {
                break;
            }
        }
    }

    fn trigger_nova(&mut self) {
        let rad = 145.0 + self.nova_lvl as f32 * 28.0;
        self.spawn_ring(self.player_pos, rad, Color::from_rgba(184, 41, 234, 255), 0.35);
        for i in 0..self.enemies.len() {
            if !self.enemies[i].active { continue; }
            let diff = self.enemies[i].pos - self.player_pos;
            let d = diff.length();
            if d < rad + self.enemies[i].radius {
                if d > 0.1 {
                    self.enemies[i].pos += (diff / d) * 95.0;
                }
                let epos = self.enemies[i].pos;
                let dmg = self.bullet_dmg * (1.1 + self.nova_lvl as f32 * 0.35);
                self.damage_enemy(i, dmg, epos, true);
            }
        }
    }

    fn damage_enemy(&mut self, idx: usize, dmg: f32, hit_pos: Vec2, show_num: bool) {
        self.enemies[idx].hp -= dmg;
        self.enemies[idx].flash = 0.06;
        if show_num && self.settings.dmg_nums {
            let col = if dmg >= 35.0 { Color::from_rgba(249, 200, 14, 255) } else { WHITE };
            let pos = self.enemies[idx].pos + vec2(rand::gen_range(-8.0, 8.0), -self.enemies[idx].radius);
            self.spawn_text(pos, format!("{}", dmg.ceil() as i32), col);
        }

        if self.enemies[idx].hp <= 0.0 && self.enemies[idx].active {
            self.enemies[idx].active = false;
            self.kills += 1;
            let kind = self.enemies[idx].kind;
            let epos = self.enemies[idx].pos;
            let erad = self.enemies[idx].radius;

            self.sounds.play(&self.sounds.boom, self.settings.sound, if kind == 3 { 1.0 } else { 0.6 });
            if self.settings.shake {
                self.shake = self.shake.max(if kind == 3 { 12.0 } else { 3.5 });
            }

            let col = match kind {
                1 => Color::from_rgba(255, 123, 0, 255),
                2 => Color::from_rgba(249, 200, 14, 255),
                3 => Color::from_rgba(184, 41, 234, 255),
                _ => Color::from_rgba(255, 42, 109, 255),
            };
            self.spawn_ring(epos, erad * 2.6, col, 0.25);

            let count = if kind == 3 { 28 } else { 10 };
            for _ in 0..count {
                let a = rand::gen_range(0.0, std::f32::consts::TAU);
                let spd = rand::gen_range(70.0, 290.0);
                self.spawn_particle(epos, Vec2::from_angle(a) * spd, rand::gen_range(0.25, 0.5), 3.5, col);
            }

            if kind == 3 {
                for _ in 0..4 {
                    let off = vec2(rand::gen_range(-24.0, 24.0), rand::gen_range(-24.0, 24.0));
                    self.spawn_enemy_at(0, epos + off);
                }
            }

            let shard = if kind == 3 { 12 } else if kind == 2 { 3 } else if rand::gen_range(0, 10) < 3 { 1 } else { 0 };
            let xp = if kind == 3 { 45.0 } else if kind == 2 { 15.0 } else { 6.0 };
            self.spawn_gem(epos, xp, shard);
        } else {
            self.sounds.play(&self.sounds.hit, self.settings.sound, 0.35);
            self.spawn_particle(
                hit_pos,
                vec2(rand::gen_range(-70.0, 70.0), rand::gen_range(-70.0, 70.0)),
                0.14,
                2.5,
                WHITE,
            );
        }
    }

    fn roll_levelup(&mut self) {
        let mut pool = ALL_UPGRADES;
        for i in (1..pool.len()).rev() {
            let j = rand::gen_range(0, i + 1);
            pool.swap(i, j);
        }
        self.level_choices = [pool[0], pool[1], pool[2]];
        self.state = ScreenState::LevelUp;
    }

    fn apply_upgrade(&mut self, id: u8) {
        match id {
            0 => self.bullet_count += 1,
            1 => self.bullet_dmg *= 1.25,
            2 => self.fire_interval *= 0.83,
            3 => self.bullet_pierce += 1,
            4 => { self.orbit_count += 1; self.orbit_dmg += 6.0; }
            5 => self.tesla_lvl += 1,
            6 => self.nova_lvl += 1,
            7 => self.dash_max_cd *= 0.75,
            8 => { self.max_hp += 25.0; self.hp = (self.hp + self.max_hp * 0.4).min(self.max_hp); }
            9 => self.magnet_radius *= 1.45,
            _ => {}
        }
        self.state = ScreenState::Playing;
    }

    fn finish_run(&mut self, victory: bool) {
        self.meta.shards += self.run_shards;
        self.meta.save();
        self.state = ScreenState::GameOver { victory };
    }

    fn find_nearest_enemy(&self, origin: Vec2, max_range: f32) -> Option<Vec2> {
        let mut best = None;
        let mut best_d = max_range;
        for e in &self.enemies {
            if !e.active { continue; }
            let d = e.pos.distance(origin);
            if d < best_d {
                best_d = d;
                best = Some(e.pos);
            }
        }
        best
    }

    fn spawn_bullet(&mut self, pos: Vec2, vel: Vec2, dmg: f32, pierce: i32) {
        for b in &mut self.bullets {
            if !b.active {
                b.active = true; b.pos = pos; b.vel = vel; b.dmg = dmg; b.life = 1.1; b.pierce = pierce;
                return;
            }
        }
    }

    fn spawn_enemy(&mut self, kind: u8) {
        let angle = rand::gen_range(0.0, std::f32::consts::TAU);
        let dist = (screen_width().max(screen_height()) * 0.62 + 70.0).min(ARENA_RADIUS - 60.0);
        let mut pos = self.player_pos + Vec2::from_angle(angle) * dist;
        if pos.length() > ARENA_RADIUS - 45.0 {
            pos = pos.normalize() * (ARENA_RADIUS - 45.0);
        }
        self.spawn_enemy_at(kind, pos);
    }

    fn spawn_enemy_at(&mut self, kind: u8, pos: Vec2) {
        for e in &mut self.enemies {
            if !e.active {
                let scale = 1.0 + (self.wave - 1) as f32 * 0.24;
                e.active = true; e.kind = kind; e.pos = pos;
                e.flash = 0.0; e.timer = rand::gen_range(0.0, 2.0);
                e.angle = rand::gen_range(0.0, std::f32::consts::TAU);
                e.spin = rand::gen_range(-3.5, 3.5);
                match kind {
                    0 => { e.max_hp = 22.0 * scale; e.speed = 115.0 + self.wave as f32 * 3.0; e.radius = 12.0; }
                    1 => { e.max_hp = 32.0 * scale; e.speed = 135.0 + self.wave as f32 * 4.0; e.radius = 13.0; }
                    2 => { e.max_hp = 115.0 * scale; e.speed = 68.0; e.radius = 22.0; }
                    _ => { e.max_hp = 500.0 * scale; e.speed = 62.0; e.radius = 35.0; }
                }
                e.hp = e.max_hp;
                return;
            }
        }
    }

    fn spawn_particle(&mut self, pos: Vec2, vel: Vec2, life: f32, size: f32, color: Color) {
        for p in &mut self.particles {
            if !p.active {
                p.active = true; p.pos = pos; p.vel = vel; p.life = life; p.max_life = life; p.size = size; p.color = color;
                return;
            }
        }
    }

    fn spawn_ring(&mut self, pos: Vec2, max_radius: f32, color: Color, life: f32) {
        for r in &mut self.rings {
            if !r.active {
                r.active = true; r.pos = pos; r.radius = 4.0; r.max_radius = max_radius; r.life = life; r.max_life = life; r.color = color;
                return;
            }
        }
    }

    fn spawn_text(&mut self, pos: Vec2, val: String, color: Color) {
        for t in &mut self.texts {
            if !t.active {
                t.active = true; t.pos = pos; t.val = val; t.life = 0.55; t.color = color;
                return;
            }
        }
    }

    fn spawn_gem(&mut self, pos: Vec2, xp: f32, shard: u32) {
        for g in &mut self.gems {
            if !g.active {
                g.active = true; g.pos = pos; g.xp = xp; g.shard = shard;
                return;
            }
        }
    }

    fn spawn_arc(&mut self, a: Vec2, b: Vec2) {
        for arc in &mut self.arcs {
            if !arc.active {
                arc.active = true; arc.a = a; arc.b = b; arc.life = 0.14;
                return;
            }
        }
    }

    fn render_world(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let sx = if self.shake > 0.0 { rand::gen_range(-self.shake, self.shake) } else { 0.0 };
        let sy = if self.shake > 0.0 { rand::gen_range(-self.shake, self.shake) } else { 0.0 };

        // Parallax starfield in screen space
        for (pos, z, size) in &self.stars {
            let x = ((pos.x - self.player_pos.x * *z).rem_euclid(3200.0)) - 800.0;
            let y = ((pos.y - self.player_pos.y * *z).rem_euclid(3200.0)) - 800.0;
            if x >= -5.0 && x <= sw + 5.0 && y >= -5.0 && y <= sh + 5.0 {
                draw_rectangle(x, y, *size, *size, Color::from_rgba(140, 210, 255, 110));
            }
        }

        let cam_rect = Rect::new(self.player_pos.x - sw * 0.5 + sx, self.player_pos.y - sh * 0.5 + sy, sw, sh);
        set_camera(&Camera2D::from_display_rect(cam_rect));

        // Cyber grid
        let grid = 80.0;
        let start_x = ((cam_rect.x / grid).floor() * grid).max(-ARENA_RADIUS);
        let end_x = (cam_rect.x + sw).min(ARENA_RADIUS);
        let start_y = ((cam_rect.y / grid).floor() * grid).max(-ARENA_RADIUS);
        let end_y = (cam_rect.y + sh).min(ARENA_RADIUS);
        let grid_col = Color::from_rgba(5, 217, 232, 18);

        let mut gx = start_x;
        while gx <= end_x {
            draw_line(gx, start_y, gx, end_y, 1.0, grid_col);
            gx += grid;
        }
        let mut gy = start_y;
        while gy <= end_y {
            draw_line(start_x, gy, end_x, gy, 1.0, grid_col);
            gy += grid;
        }

        // Arena wall (double neon stroke)
        draw_circle_lines(0.0, 0.0, ARENA_RADIUS, 12.0, Color::from_rgba(255, 42, 109, 55));
        draw_circle_lines(0.0, 0.0, ARENA_RADIUS, 2.5, Color::from_rgba(255, 42, 109, 255));

        // Rings
        for r in &self.rings {
            if !r.active { continue; }
            let mut c = r.color;
            c.a = (r.life / r.max_life).clamp(0.0, 1.0);
            draw_circle_lines(r.pos.x, r.pos.y, r.radius, 3.0, c);
        }

        // Gems
        for g in &self.gems {
            if !g.active { continue; }
            if g.shard > 0 {
                draw_circle(g.pos.x, g.pos.y, 5.5, Color::from_rgba(249, 200, 14, 255));
            } else {
                draw_circle(g.pos.x, g.pos.y, 3.5, Color::from_rgba(5, 217, 232, 255));
            }
        }

        // Bullets
        for b in &self.bullets {
            if !b.active { continue; }
            let tail = b.pos - b.vel * 0.028;
            draw_line(b.pos.x, b.pos.y, tail.x, tail.y, 7.0, Color::from_rgba(5, 217, 232, 85));
            draw_line(b.pos.x, b.pos.y, tail.x, tail.y, 2.5, WHITE);
        }

        // Tesla arcs
        for a in &self.arcs {
            if !a.active { continue; }
            let mid = (a.a + a.b) * 0.5 + vec2(rand::gen_range(-12.0, 12.0), rand::gen_range(-12.0, 12.0));
            let col = Color::from_rgba(125, 249, 255, 255);
            draw_line(a.a.x, a.a.y, mid.x, mid.y, 2.5, col);
            draw_line(mid.x, mid.y, a.b.x, a.b.y, 2.5, col);
        }

        // Enemies
        for e in &self.enemies {
            if !e.active { continue; }
            let base_col = if e.flash > 0.0 {
                WHITE
            } else {
                match e.kind {
                    1 => Color::from_rgba(255, 123, 0, 255),
                    2 => Color::from_rgba(249, 200, 14, 255),
                    3 => Color::from_rgba(184, 41, 234, 255),
                    _ => Color::from_rgba(255, 42, 109, 255),
                }
            };
            let sides = match e.kind { 0 => 3, 1 => 4, 2 => 6, _ => 8 };
            let mut glow_col = base_col;
            glow_col.a = 0.28;
            let deg = e.angle.to_degrees();
            draw_poly_lines(e.pos.x, e.pos.y, sides, e.radius, deg, 7.0, glow_col);
            draw_poly_lines(e.pos.x, e.pos.y, sides, e.radius, deg, 2.2, base_col);

            if (e.kind == 2 || e.kind == 3) && e.hp < e.max_hp {
                let bw = e.radius * 2.0;
                let ratio = (e.hp / e.max_hp).clamp(0.0, 1.0);
                draw_rectangle(e.pos.x - e.radius, e.pos.y - e.radius - 10.0, bw, 4.0, Color::from_rgba(255, 255, 255, 50));
                draw_rectangle(e.pos.x - e.radius, e.pos.y - e.radius - 10.0, bw * ratio, 4.0, base_col);
            }
        }

        // Orbital saws
        if self.orbit_count > 0 {
            let col = Color::from_rgba(5, 255, 161, 255);
            for i in 0..self.orbit_count {
                let a = self.orbit_angle + (i as f32 * std::f32::consts::TAU) / self.orbit_count as f32;
                let bpos = self.player_pos + Vec2::from_angle(a) * 72.0;
                draw_circle_lines(bpos.x, bpos.y, 8.5, 2.5, col);
            }
        }

        // Player ship
        let p = self.player_pos;
        let nose = p + Vec2::from_angle(self.aim_angle) * 20.0;
        let left = p + Vec2::from_angle(self.aim_angle + 2.4) * 15.0;
        let tail = p + Vec2::from_angle(self.aim_angle + std::f32::consts::PI) * 7.0;
        let right = p + Vec2::from_angle(self.aim_angle - 2.4) * 15.0;

        let ship_col = if self.dash_timer > 0.0 { WHITE } else { Color::from_rgba(5, 217, 232, 255) };
        let glow = Color::from_rgba(5, 217, 232, 85);
        for &(a, b) in &[(nose, left), (left, tail), (tail, right), (right, nose)] {
            draw_line(a.x, a.y, b.x, b.y, 7.5, glow);
            draw_line(a.x, a.y, b.x, b.y, 2.5, ship_col);
        }

        // Off-screen enemy radar dots
        for (idx, e) in self.enemies.iter().enumerate() {
            if !e.active { continue; }
            let diff = e.pos - p;
            if diff.x.abs() > sw * 0.48 || diff.y.abs() > sh * 0.48 {
                if e.kind == 3 || idx % 4 == 0 {
                    let ind = p + diff.normalize() * 46.0;
                    let c = if e.kind == 3 {
                        Color::from_rgba(184, 41, 234, 255)
                    } else {
                        Color::from_rgba(255, 42, 109, 170)
                    };
                    draw_circle(ind.x, ind.y, if e.kind == 3 { 4.5 } else { 2.5 }, c);
                }
            }
        }

        // Particles
        for pt in &self.particles {
            if !pt.active { continue; }
            let mut c = pt.color;
            c.a = (pt.life / pt.max_life).clamp(0.0, 1.0);
            draw_rectangle(pt.pos.x - pt.size * 0.5, pt.pos.y - pt.size * 0.5, pt.size, pt.size, c);
        }

        // Damage numbers
        for t in &self.texts {
            if !t.active { continue; }
            let mut c = t.color;
            c.a = (t.life * 2.0).clamp(0.0, 1.0);
            draw_text(&t.val, t.pos.x - 10.0, t.pos.y, 18.0, c);
        }

        set_default_camera();
    }

    fn render_hud(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let cyan = Color::from_rgba(5, 217, 232, 255);
        let pink = Color::from_rgba(255, 42, 109, 255);
        let gold = Color::from_rgba(249, 200, 14, 255);

        // HP & XP bars
        let bar_w = (sw * 0.34).min(175.0);
        draw_text(&format!("HULL {}/{}", self.hp.ceil() as i32, self.max_hp as i32), 14.0, 24.0, 18.0, WHITE);
        draw_rectangle(14.0, 30.0, bar_w, 9.0, Color::from_rgba(255, 255, 255, 25));
        draw_rectangle(14.0, 30.0, bar_w * (self.hp / self.max_hp).clamp(0.0, 1.0), 9.0, pink);
        draw_rectangle_lines(14.0, 30.0, bar_w, 9.0, 1.0, cyan);

        draw_text(&format!("LVL {}", self.level), 14.0, 56.0, 18.0, WHITE);
        draw_rectangle(14.0, 62.0, bar_w, 9.0, Color::from_rgba(255, 255, 255, 25));
        draw_rectangle(14.0, 62.0, bar_w * (self.xp / self.next_xp).clamp(0.0, 1.0), 9.0, cyan);
        draw_rectangle_lines(14.0, 62.0, bar_w, 9.0, 1.0, cyan);

        // Center Wave info
        draw_centered_text(&format!("WAVE {}", self.wave), sw * 0.5, 26.0, 22.0, pink);
        draw_centered_text(&format!("{}s", (20.0 - self.wave_timer).ceil().max(1.0) as i32), sw * 0.5, 46.0, 16.0, LIGHTGRAY);

        // Right Shards & FPS
        draw_text(&format!("SHARDS: {}", self.run_shards), sw - 165.0, 24.0, 18.0, gold);
        draw_text(&format!("{} FPS (RUST)", get_fps()), sw - 165.0, 44.0, 14.0, LIGHTGRAY);

        // Pause button
        let pause_rect = Rect::new(sw - 54.0, 12.0, 42.0, 42.0);
        draw_rectangle(pause_rect.x, pause_rect.y, pause_rect.w, pause_rect.h, Color::from_rgba(10, 16, 30, 220));
        draw_rectangle_lines(pause_rect.x, pause_rect.y, pause_rect.w, pause_rect.h, 1.5, cyan);
        draw_centered_text("||", pause_rect.x + 21.0, pause_rect.y + 28.0, 20.0, cyan);

        // Dash button
        let dc = vec2(sw - 58.0, sh - 62.0);
        let dash_col = if self.dash_cd > 0.0 { Color::from_rgba(70, 90, 120, 180) } else { cyan };
        draw_circle(dc.x, dc.y, 35.0, Color::from_rgba(5, 217, 232, 25));
        draw_circle_lines(dc.x, dc.y, 35.0, 2.0, dash_col);
        draw_centered_text("DASH", dc.x, dc.y - 2.0, 17.0, dash_col);
        let cd_label = if self.dash_cd > 0.0 { format!("{:.1}s", self.dash_cd) } else { "READY".to_string() };
        draw_centered_text(&cd_label, dc.x, dc.y + 15.0, 13.0, dash_col);

        // Joysticks
        if self.left_stick.touch_id.is_none() {
            draw_circle_lines(86.0, sh - 96.0, 42.0, 2.0, Color::from_rgba(5, 217, 232, 65));
            draw_circle(86.0, sh - 96.0, 12.0, Color::from_rgba(5, 217, 232, 45));
        } else {
            draw_circle_lines(self.left_stick.start.x, self.left_stick.start.y, 44.0, 2.0, cyan);
            draw_circle(self.left_stick.cur.x, self.left_stick.cur.y, 16.0, cyan);
        }

        if self.right_stick.touch_id.is_some() {
            draw_circle_lines(self.right_stick.start.x, self.right_stick.start.y, 44.0, 2.0, pink);
            draw_circle(self.right_stick.cur.x, self.right_stick.cur.y, 16.0, pink);
        }
    }
}

fn draw_centered_text(text: &str, cx: f32, y: f32, font_size: f32, color: Color) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    draw_text(text, cx - dims.width * 0.5, y, font_size, color);
}

fn ui_button(rect: Rect, label: &str, sub: Option<&str>, border: Color) -> bool {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(11, 17, 32, 240));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
    if let Some(sub_txt) = sub {
        draw_centered_text(label, rect.x + rect.w * 0.5, rect.y + 24.0, 18.0, border);
        draw_centered_text(sub_txt, rect.x + rect.w * 0.5, rect.y + 44.0, 14.0, LIGHTGRAY);
    } else {
        draw_centered_text(label, rect.x + rect.w * 0.5, rect.y + rect.h * 0.5 + 6.0, 20.0, border);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        return rect.contains(vec2(mx, my));
    }
    false
}

fn window_conf() -> Conf {
    Conf {
        window_title: "NEON CORE [RUST]".to_owned(),
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let sounds = SoundBank::new().await;
    let mut game = Game::new(sounds);

    let cyan = Color::from_rgba(5, 217, 232, 255);
    let pink = Color::from_rgba(255, 42, 109, 255);
    let gold = Color::from_rgba(249, 200, 14, 255);

    loop {
        let dt = get_frame_time().min(0.05);
        clear_background(Color::from_rgba(4, 6, 11, 255));

        let sw = screen_width();
        let sh = screen_height();

        match game.state {
            ScreenState::Menu => {
                let badge = if cfg!(feature = "demo") { "RUST WASM DEMO (MAX WAVE 8)" } else { "NATIVE RUST ARM64 • ZERO ADS" };
                draw_centered_text(badge, sw * 0.5, 42.0, 16.0, pink);
                draw_centered_text("NEON CORE", sw * 0.5, 86.0, 46.0, cyan);
                draw_centered_text(&format!("BANKED SHARDS: {}", game.meta.shards), sw * 0.5, 118.0, 20.0, gold);

                let cols = if sw > 620.0 { 2 } else { 1 };
                let card_w = if cols == 2 { 270.0 } else { (sw - 40.0).min(340.0) };
                let card_h = 56.0;
                let start_x = sw * 0.5 - (cols as f32 * card_w + (cols - 1) as f32 * 12.0) * 0.5;
                let start_y = 140.0;

                let upgrades = [
                    ("MOC RDZENIA", game.meta.dmg_lvl, 0),
                    ("SZYBKOSTRZELNOSC", game.meta.rate_lvl, 1),
                    ("PANCERZ KADŁUBA", game.meta.hp_lvl, 2),
                    ("SILNIKI MANEWROWE", game.meta.spd_lvl, 3),
                ];

                for (idx, (name, lvl, kind)) in upgrades.iter().enumerate() {
                    let r = idx / cols;
                    let c = idx % cols;
                    let rx = start_x + c as f32 * (card_w + 12.0);
                    let ry = start_y + r as f32 * (card_h + 10.0);
                    let cost = (*lvl + 1) * 15;
                    let title = format!("{} [LVL {}]", name, lvl);
                    let sub = format!("KOSZT: {} SHARDS", cost);
                    if ui_button(Rect::new(rx, ry, card_w, card_h), &title, Some(&sub), Color::from_rgba(45, 95, 155, 255)) {
                        if game.meta.shards >= cost {
                            game.meta.shards -= cost;
                            match kind {
                                0 => game.meta.dmg_lvl += 1,
                                1 => game.meta.rate_lvl += 1,
                                2 => game.meta.hp_lvl += 1,
                                _ => game.meta.spd_lvl += 1,
                            }
                            game.meta.save();
                            game.sounds.play(&game.sounds.lvlup, game.settings.sound, 0.8);
                        }
                    }
                }

                let btn_y = start_y + (4 / cols) as f32 * (card_h + 10.0) + 14.0;
                if ui_button(Rect::new(sw * 0.5 - 155.0, btn_y, 148.0, 48.0), "START GRY", None, cyan) {
                    game.start_run();
                }
                if ui_button(Rect::new(sw * 0.5 + 7.0, btn_y, 148.0, 48.0), "OPCJE", None, gold) {
                    game.prev_state = ScreenState::Menu;
                    game.state = ScreenState::Settings;
                }
            }
            ScreenState::Playing => {
                game.update_playing(dt);
                game.render_world();
                game.render_hud();
            }
            ScreenState::Paused => {
                game.render_world();
                draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(4, 6, 11, 210));
                draw_centered_text("PAUZA", sw * 0.5, sh * 0.28, 38.0, cyan);
                if ui_button(Rect::new(sw * 0.5 - 120.0, sh * 0.38, 240.0, 48.0), "WZNOW", None, cyan) {
                    game.state = ScreenState::Playing;
                }
                if ui_button(Rect::new(sw * 0.5 - 120.0, sh * 0.38 + 60.0, 240.0, 48.0), "OPCJE", None, gold) {
                    game.prev_state = ScreenState::Paused;
                    game.state = ScreenState::Settings;
                }
                if ui_button(Rect::new(sw * 0.5 - 120.0, sh * 0.38 + 120.0, 240.0, 48.0), "PRZERWIJ LOT", None, pink) {
                    game.finish_run(false);
                }
            }
            ScreenState::Settings => {
                draw_centered_text("OPCJE SYSTEMU", sw * 0.5, 65.0, 32.0, cyan);
                let bw = (sw - 40.0).min(320.0);
                let bx = sw * 0.5 - bw * 0.5;

                let s_lbl = if game.settings.sound { "DZWIEK: WLACZONY" } else { "DZWIEK: WYLACZONY" };
                if ui_button(Rect::new(bx, 105.0, bw, 44.0), s_lbl, None, cyan) {
                    game.settings.sound = !game.settings.sound;
                }
                let sh_lbl = if game.settings.shake { "SHAKE: WLACZONY" } else { "SHAKE: WYLACZONY" };
                if ui_button(Rect::new(bx, 158.0, bw, 44.0), sh_lbl, None, cyan) {
                    game.settings.shake = !game.settings.shake;
                }
                let dm_lbl = if game.settings.dmg_nums { "OBRAZENIA: WLACZONE" } else { "OBRAZENIA: WYLACZONE" };
                if ui_button(Rect::new(bx, 211.0, bw, 44.0), dm_lbl, None, cyan) {
                    game.settings.dmg_nums = !game.settings.dmg_nums;
                }
                let st_lbl = if game.settings.fixed_stick { "DRAZEK: STALY" } else { "DRAZEK: PLYWAJACY" };
                if ui_button(Rect::new(bx, 264.0, bw, 44.0), st_lbl, None, cyan) {
                    game.settings.fixed_stick = !game.settings.fixed_stick;
                }
                let ai_lbl = if game.settings.auto_aim { "AUTO-AIM: WLACZONY" } else { "AUTO-AIM: WYLACZONY" };
                if ui_button(Rect::new(bx, 317.0, bw, 44.0), ai_lbl, None, cyan) {
                    game.settings.auto_aim = !game.settings.auto_aim;
                }
                if ui_button(Rect::new(bx, 378.0, bw, 48.0), "WROC", None, gold) {
                    game.state = game.prev_state;
                }
            }
            ScreenState::LevelUp => {
                game.render_world();
                draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(4, 6, 11, 215));
                draw_centered_text("OVERCLOCK RDZENIA", sw * 0.5, 65.0, 30.0, cyan);
                let cw = (sw - 40.0).min(340.0);
                for (i, opt) in game.level_choices.iter().enumerate() {
                    let rect = Rect::new(sw * 0.5 - cw * 0.5, 105.0 + i as f32 * 74.0, cw, 62.0);
                    if ui_button(rect, opt.title, Some(opt.desc), cyan) {
                        game.apply_upgrade(opt.id);
                        break;
                    }
                }
            }
            ScreenState::GameOver { victory } => {
                let title = if victory { "SEKTOR DEMO OCZYSZCZONY" } else { "RDZEN ZNISZCZONY" };
                draw_centered_text(title, sw * 0.5, sh * 0.3, 32.0, if victory { cyan } else { pink });
                draw_centered_text(
                    &format!("FALA: {}  |  POZIOM: {}", game.wave, game.level),
                    sw * 0.5,
                    sh * 0.3 + 44.0,
                    20.0,
                    WHITE,
                );
                draw_centered_text(
                    &format!("ZNISZCZENI: {}  |  SHARDS: +{}", game.kills, game.run_shards),
                    sw * 0.5,
                    sh * 0.3 + 74.0,
                    20.0,
                    gold,
                );
                if ui_button(Rect::new(sw * 0.5 - 135.0, sh * 0.3 + 115.0, 270.0, 50.0), "POWROT DO HANGARU", None, cyan) {
                    game.state = ScreenState::Menu;
                }
            }
        }

        next_frame().await;
    }
}
