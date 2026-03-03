use fixed::types::I32F32;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

// ============================================================================
// Fixed-point type alias (s32.32 — matches FixPointCS Fixed64)
// ============================================================================

type Fp = I32F32;

// ============================================================================
// Game Constants (all Fixed-point)
// ============================================================================

fn canvas_width() -> Fp { Fp::from_num(800) }
fn canvas_height() -> Fp { Fp::from_num(600) }
fn paddle_width() -> Fp { Fp::from_num(120) }
fn paddle_height() -> Fp { Fp::from_num(14) }
fn paddle_y() -> Fp { canvas_height() - Fp::from_num(40) }
fn paddle_speed() -> Fp { Fp::from_num(600) }
fn ball_radius() -> Fp { Fp::from_num(8) }
fn ball_initial_speed() -> Fp { Fp::from_num(350) }
fn ball_speed_increment() -> Fp { Fp::from_num(8) }
fn ball_max_speed() -> Fp { Fp::from_num(700) }
fn brick_width() -> Fp { Fp::from_num(58) }
fn brick_height() -> Fp { Fp::from_num(22) }
fn brick_padding() -> Fp { Fp::from_num(6) }
fn brick_offset_top() -> Fp { Fp::from_num(60) }
fn brick_offset_left() -> Fp {
    let total = Fp::from_num(BRICK_COLS as i32) * (brick_width() + brick_padding()) - brick_padding();
    (canvas_width() - total) / Fp::from_num(2)
}

const BALL_TRAIL_LENGTH: usize = 12;
const BRICK_ROWS: usize = 6;
const BRICK_COLS: usize = 12;
const MAX_LIVES: u32 = 3;
const MAX_PARTICLES: usize = 500;

const ROW_COLORS: [&str; 6] = [
    "#ff3366", "#ff6633", "#ffcc00", "#33ff66", "#33ccff", "#9966ff",
];
const ROW_GLOW: [&str; 6] = [
    "rgba(255,51,102,0.5)", "rgba(255,102,51,0.5)", "rgba(255,204,0,0.5)",
    "rgba(51,255,102,0.5)", "rgba(51,204,255,0.5)", "rgba(153,102,255,0.5)",
];

// Fixed-point constants
fn fp_zero() -> Fp { Fp::from_num(0) }
fn fp_one() -> Fp { Fp::from_num(1) }
fn fp_two() -> Fp { Fp::from_num(2) }
fn fp_half() -> Fp { Fp::from_num(0.5) }
fn fp_tau() -> Fp { Fp::from_num(std::f64::consts::TAU) }
fn fp_frac_pi_2() -> Fp { Fp::from_num(std::f64::consts::FRAC_PI_2) }
fn fp_frac_pi_3() -> Fp { Fp::from_num(std::f64::consts::FRAC_PI_3) }
fn fp_frac_pi_4() -> Fp { Fp::from_num(std::f64::consts::FRAC_PI_4) }

// ============================================================================
// Game Types
// ============================================================================

#[derive(Clone, Copy)]
struct Vec2 { x: Fp, y: Fp }

impl Vec2 {
    fn new(x: Fp, y: Fp) -> Self { Self { x, y } }

    fn normalize(&self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y;
        if len_sq == fp_zero() { return *self; }
        let len = cordic::sqrt(len_sq);
        Self { x: self.x / len, y: self.y / len }
    }
}

#[derive(Clone, Copy)]
struct Brick { x: Fp, y: Fp, alive: bool, row: usize }

#[derive(Clone, Copy)]
struct Particle {
    x: Fp, y: Fp, vx: Fp, vy: Fp,
    life: Fp, max_life: Fp, size: Fp,
    color_index: usize, active: bool,
}

impl Particle {
    fn new() -> Self {
        Self {
            x: fp_zero(), y: fp_zero(),
            vx: fp_zero(), vy: fp_zero(),
            life: fp_zero(), max_life: fp_zero(),
            size: fp_zero(), color_index: 0, active: false,
        }
    }
}

#[derive(Clone, Copy)]
struct Trail { x: Fp, y: Fp, alpha: Fp }

#[derive(PartialEq, Clone, Copy)]
enum GameState { Wait, Play, Over, Won }

// ============================================================================
// Pseudo-Random Number Generator (returns Fp in [0, 1))
// ============================================================================

fn prand(seed: u64) -> Fp {
    let mut v = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    v ^= v >> 22;
    v ^= v << 13;
    v ^= v >> 8;
    // Return a value in [0, 1): (v % 10000) / 10000
    let numerator = Fp::from_num((v % 10000) as i32);
    let denominator = Fp::from_num(10000);
    numerator / denominator
}

// ============================================================================
// Game
// ============================================================================

struct Game {
    paddle_x: Fp,
    ball_pos: Vec2,
    ball_vel: Vec2,
    ball_speed: Fp,
    bricks: Vec<Brick>,
    particles: Vec<Particle>,
    trail: Vec<Trail>,
    score: u32,
    lives: u32,
    state: GameState,
    left_pressed: bool,
    right_pressed: bool,
    last_time: f64,
    bricks_broken: u32,
    bricks_total: u32,
    frame_count: u64,
}

impl Game {
    fn new() -> Self {
        let mut game = Game {
            paddle_x: canvas_width() / fp_two() - paddle_width() / fp_two(),
            ball_pos: Vec2::new(
                canvas_width() / fp_two(),
                paddle_y() - ball_radius() - fp_two(),
            ),
            ball_vel: Vec2::new(fp_zero(), fp_zero()),
            ball_speed: ball_initial_speed(),
            bricks: Vec::new(),
            particles: vec![Particle::new(); MAX_PARTICLES],
            trail: Vec::with_capacity(BALL_TRAIL_LENGTH),
            score: 0, lives: MAX_LIVES,
            state: GameState::Wait,
            left_pressed: false, right_pressed: false,
            last_time: 0.0,
            bricks_broken: 0, bricks_total: 0,
            frame_count: 0,
        };
        game.init_bricks();
        game
    }

    fn init_bricks(&mut self) {
        self.bricks.clear();
        let ol = brick_offset_left();
        let ot = brick_offset_top();
        let bw = brick_width();
        let bh = brick_height();
        let bp = brick_padding();
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let x = ol + Fp::from_num(col as i32) * (bw + bp);
                let y = ot + Fp::from_num(row as i32) * (bh + bp);
                self.bricks.push(Brick { x, y, alive: true, row });
            }
        }
        self.bricks_total = (BRICK_ROWS * BRICK_COLS) as u32;
    }

    fn reset(&mut self) {
        self.paddle_x = canvas_width() / fp_two() - paddle_width() / fp_two();
        self.ball_pos = Vec2::new(
            canvas_width() / fp_two(),
            paddle_y() - ball_radius() - fp_two(),
        );
        self.ball_vel = Vec2::new(fp_zero(), fp_zero());
        self.ball_speed = ball_initial_speed();
        self.score = 0; self.lives = MAX_LIVES; self.bricks_broken = 0;
        self.state = GameState::Wait;
        self.trail.clear();
        self.init_bricks();
        for p in &mut self.particles { p.active = false; }
    }

    fn launch(&mut self) {
        let r = prand(self.frame_count);
        let angle = -fp_frac_pi_4() + r * fp_frac_pi_2();
        self.ball_vel = Vec2::new(cordic::sin(angle), -cordic::cos(angle));
        self.state = GameState::Play;
    }

    fn reset_ball(&mut self) {
        self.ball_pos = Vec2::new(
            self.paddle_x + paddle_width() / fp_two(),
            paddle_y() - ball_radius() - fp_two(),
        );
        self.ball_vel = Vec2::new(fp_zero(), fp_zero());
        self.ball_speed = (ball_initial_speed()
            + Fp::from_num(self.bricks_broken as i32) * (ball_speed_increment() * fp_half()))
            .min(ball_max_speed());
        self.trail.clear();
        self.state = GameState::Wait;
    }

    fn spawn_particles(&mut self, x: Fp, y: Fp, ci: usize, count: usize) {
        let tau = fp_tau();
        let mut s = 0;
        for p in &mut self.particles {
            if !p.active && s < count {
                let angle = prand(self.frame_count + s as u64 * 7) * tau;
                let speed = Fp::from_num(50)
                    + prand(self.frame_count + s as u64 * 13) * Fp::from_num(200);
                p.x = x; p.y = y;
                p.vx = cordic::cos(angle) * speed;
                p.vy = cordic::sin(angle) * speed;
                p.life = fp_half()
                    + prand(self.frame_count + s as u64 * 19) * Fp::from_num(0.7);
                p.max_life = p.life;
                p.size = fp_two()
                    + prand(self.frame_count + s as u64 * 23) * Fp::from_num(4);
                p.color_index = ci;
                p.active = true;
                s += 1;
            }
        }
    }

    fn update(&mut self, dt: Fp) {
        self.frame_count += 1;
        let gravity = Fp::from_num(150);

        // Update particles
        for p in &mut self.particles {
            if p.active {
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                p.vy += gravity * dt;
                p.life -= dt;
                if p.life <= fp_zero() { p.active = false; }
            }
        }

        if self.state != GameState::Play {
            if self.state == GameState::Wait {
                self.ball_pos.x = self.paddle_x + paddle_width() / fp_two();
                self.ball_pos.y = paddle_y() - ball_radius() - fp_two();
            }
            return;
        }

        // Paddle movement
        if self.left_pressed { self.paddle_x -= paddle_speed() * dt; }
        if self.right_pressed { self.paddle_x += paddle_speed() * dt; }
        self.paddle_x = self.paddle_x.clamp(fp_zero(), canvas_width() - paddle_width());

        // Ball trail
        self.trail.push(Trail { x: self.ball_pos.x, y: self.ball_pos.y, alpha: fp_one() });
        if self.trail.len() > BALL_TRAIL_LENGTH { self.trail.remove(0); }
        let tl = self.trail.len();
        for (i, t) in self.trail.iter_mut().enumerate() {
            t.alpha = Fp::from_num((i + 1) as i32)
                / Fp::from_num(tl as i32)
                * Fp::from_num(0.6);
        }

        // Ball movement
        self.ball_pos.x += self.ball_vel.x * self.ball_speed * dt;
        self.ball_pos.y += self.ball_vel.y * self.ball_speed * dt;

        let br = ball_radius();
        let cw = canvas_width();
        let ch = canvas_height();

        // Wall collisions
        if self.ball_pos.x - br <= fp_zero() {
            self.ball_pos.x = br;
            self.ball_vel.x = self.ball_vel.x.abs();
        }
        if self.ball_pos.x + br >= cw {
            self.ball_pos.x = cw - br;
            self.ball_vel.x = -self.ball_vel.x.abs();
        }
        if self.ball_pos.y - br <= fp_zero() {
            self.ball_pos.y = br;
            self.ball_vel.y = self.ball_vel.y.abs();
        }

        // Ball out at bottom
        if self.ball_pos.y + br >= ch {
            self.lives -= 1;
            if self.lives == 0 { self.state = GameState::Over; }
            else { self.reset_ball(); }
            return;
        }

        // Paddle collision
        let py = paddle_y();
        let pw = paddle_width();
        let ph = paddle_height();
        if self.ball_vel.y > fp_zero() {
            let pl = self.paddle_x;
            let pr = self.paddle_x + pw;
            if self.ball_pos.y + br >= py
                && self.ball_pos.y + br <= py + ph + Fp::from_num(4)
                && self.ball_pos.x >= pl - br
                && self.ball_pos.x <= pr + br
            {
                let hp = (self.ball_pos.x - pl) / pw;
                let angle = (hp - fp_half()) * fp_frac_pi_3() * Fp::from_num(2.5);
                self.ball_vel = Vec2::new(cordic::sin(angle), -cordic::cos(angle)).normalize();
                self.ball_pos.y = py - br;
            }
        }

        // Brick collision
        let bw = brick_width();
        let bh = brick_height();
        let mut hit: Option<usize> = None;

        for (i, b) in self.bricks.iter().enumerate() {
            if !b.alive { continue; }
            let cx = self.ball_pos.x.clamp(b.x, b.x + bw);
            let cy = self.ball_pos.y.clamp(b.y, b.y + bh);
            let dx = self.ball_pos.x - cx;
            let dy = self.ball_pos.y - cy;
            if dx * dx + dy * dy <= br * br {
                hit = Some(i);
                let bcx = b.x + bw / fp_two();
                let bcy = b.y + bh / fp_two();
                let dfx = self.ball_pos.x - bcx;
                let dfy = self.ball_pos.y - bcy;
                if dfx.abs() / bw > dfy.abs() / bh {
                    self.ball_vel.x = if dfx > fp_zero() { self.ball_vel.x.abs() } else { -self.ball_vel.x.abs() };
                } else {
                    self.ball_vel.y = if dfy > fp_zero() { self.ball_vel.y.abs() } else { -self.ball_vel.y.abs() };
                }
                break;
            }
        }

        if let Some(i) = hit {
            let b = self.bricks[i];
            self.bricks[i].alive = false;
            self.score += (BRICK_ROWS - b.row) as u32 * 10;
            self.bricks_broken += 1;
            self.ball_speed = (self.ball_speed + ball_speed_increment()).min(ball_max_speed());
            self.spawn_particles(b.x + bw / fp_two(), b.y + bh / fp_two(), b.row, 15);
            if self.bricks_broken >= self.bricks_total { self.state = GameState::Won; }
        }
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d) {
        let cw = canvas_width().to_num::<f64>();
        let ch = canvas_height().to_num::<f64>();

        // Background
        ctx.set_fill_style_str("#0a0a2e");
        ctx.fill_rect(0.0, 0.0, cw, ch);

        // Grid
        ctx.set_stroke_style_str("rgba(255,255,255,0.03)");
        ctx.set_line_width(1.0);
        let mut gx = 0.0;
        while gx < cw { ctx.begin_path(); ctx.move_to(gx, 0.0); ctx.line_to(gx, ch); ctx.stroke(); gx += 40.0; }
        let mut gy = 0.0;
        while gy < ch { ctx.begin_path(); ctx.move_to(0.0, gy); ctx.line_to(cw, gy); ctx.stroke(); gy += 40.0; }

        // Bricks
        let bwf = brick_width().to_num::<f64>();
        let bhf = brick_height().to_num::<f64>();
        for b in &self.bricks {
            if !b.alive { continue; }
            let bx = b.x.to_num::<f64>();
            let by = b.y.to_num::<f64>();
            ctx.set_shadow_color(ROW_GLOW[b.row % 6]);
            ctx.set_shadow_blur(12.0);
            ctx.set_fill_style_str(ROW_COLORS[b.row % 6]);
            draw_rounded_rect(ctx, bx, by, bwf, bhf, 4.0);
            ctx.fill();
            ctx.set_shadow_blur(0.0);
            ctx.set_fill_style_str("rgba(255,255,255,0.25)");
            draw_rounded_rect(ctx, bx + 2.0, by + 2.0, bwf - 4.0, bhf / 3.0, 2.0);
            ctx.fill();
        }
        ctx.set_shadow_blur(0.0);
        ctx.set_shadow_color("transparent");

        // Particles
        for p in &self.particles {
            if !p.active { continue; }
            let a = (p.life / p.max_life).max(fp_zero()).to_num::<f64>();
            ctx.set_fill_style_str(&color_with_alpha(ROW_COLORS[p.color_index % 6], a));
            ctx.begin_path();
            let _ = ctx.arc(p.x.to_num::<f64>(), p.y.to_num::<f64>(), p.size.to_num::<f64>() * a, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }

        // Trail
        for t in &self.trail {
            let a = t.alpha.to_num::<f64>();
            ctx.set_fill_style_str(&format!("rgba(100,200,255,{})", a * 0.4));
            ctx.begin_path();
            let _ = ctx.arc(t.x.to_num::<f64>(), t.y.to_num::<f64>(), ball_radius().to_num::<f64>() * a, 0.0, std::f64::consts::TAU);
            ctx.fill();
        }

        // Ball
        let brf = ball_radius().to_num::<f64>();
        ctx.set_shadow_color("rgba(100,200,255,0.8)");
        ctx.set_shadow_blur(20.0);
        ctx.set_fill_style_str("#ffffff");
        ctx.begin_path();
        let _ = ctx.arc(self.ball_pos.x.to_num::<f64>(), self.ball_pos.y.to_num::<f64>(), brf, 0.0, std::f64::consts::TAU);
        ctx.fill();
        ctx.set_shadow_blur(0.0);
        ctx.set_shadow_color("transparent");

        // Paddle
        let px = self.paddle_x.to_num::<f64>();
        let pyf = paddle_y().to_num::<f64>();
        let pwf = paddle_width().to_num::<f64>();
        let phf = paddle_height().to_num::<f64>();
        ctx.set_shadow_color("rgba(0,212,255,0.6)");
        ctx.set_shadow_blur(15.0);
        {
            let gr = ctx.create_linear_gradient(px, pyf, px, pyf + phf);
            let _ = gr.add_color_stop(0.0, "#00e5ff");
            let _ = gr.add_color_stop(1.0, "#0088aa");
            ctx.set_fill_style_canvas_gradient(&gr);
        }
        draw_rounded_rect(ctx, px, pyf, pwf, phf, 7.0);
        ctx.fill();
        ctx.set_shadow_blur(0.0);
        ctx.set_shadow_color("transparent");
        ctx.set_fill_style_str("rgba(255,255,255,0.3)");
        draw_rounded_rect(ctx, px + 4.0, pyf + 2.0, pwf - 8.0, phf / 3.0, 4.0);
        ctx.fill();

        // HUD
        ctx.set_fill_style_str("rgba(255,255,255,0.9)");
        ctx.set_font("bold 18px 'Segoe UI',Arial,sans-serif");
        ctx.set_text_align("left");
        let _ = ctx.fill_text(&format!("SCORE: {}", self.score), 20.0, 30.0);
        ctx.set_text_align("right");
        let _ = ctx.fill_text(&format!("LIVES: {}", self.lives), cw - 20.0, 30.0);
        ctx.set_text_align("center");
        ctx.set_fill_style_str("rgba(255,255,255,0.3)");
        ctx.set_font("12px 'Segoe UI',Arial,sans-serif");
        let speed_range = ball_max_speed() - ball_initial_speed();
        let speed_pct = if speed_range > fp_zero() {
            ((self.ball_speed - ball_initial_speed()) / speed_range * Fp::from_num(100)).to_num::<f64>() as u32
        } else { 0 };
        let _ = ctx.fill_text(&format!("SPEED +{}%", speed_pct), cw / 2.0, 30.0);
        ctx.set_stroke_style_str("rgba(0,212,255,0.3)");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(0.0, 44.0);
        ctx.line_to(cw, 44.0);
        ctx.stroke();

        // Overlays
        match self.state {
            GameState::Wait => {
                let (t, s) = if self.lives == MAX_LIVES && self.score == 0 {
                    ("BREAKOUT", "Click or press Space to start")
                } else {
                    ("READY", "Click or press Space to launch")
                };
                draw_overlay(ctx, t, s);
            }
            GameState::Over => {
                draw_overlay(ctx, "GAME OVER", &format!("Score: {} \u{2014} Press Space to restart", self.score));
            }
            GameState::Won => {
                draw_overlay(ctx, "YOU WIN!", &format!("Score: {} \u{2014} Press Space to play again", self.score));
            }
            GameState::Play => {}
        }
    }

    fn mouse_move(&mut self, x: Fp) {
        if self.state == GameState::Play || self.state == GameState::Wait {
            self.paddle_x = (x - paddle_width() / fp_two()).clamp(fp_zero(), canvas_width() - paddle_width());
        }
    }

    fn click(&mut self) {
        match self.state {
            GameState::Wait => self.launch(),
            GameState::Over | GameState::Won => self.reset(),
            _ => {}
        }
    }

    fn key_down(&mut self, key: &str) {
        match key {
            "ArrowLeft" | "a" | "A" => self.left_pressed = true,
            "ArrowRight" | "d" | "D" => self.right_pressed = true,
            " " => self.click(),
            _ => {}
        }
    }

    fn key_up(&mut self, key: &str) {
        match key {
            "ArrowLeft" | "a" | "A" => self.left_pressed = false,
            "ArrowRight" | "d" | "D" => self.right_pressed = false,
            _ => {}
        }
    }
}

// ============================================================================
// Helper Functions (rendering — all f64)
// ============================================================================

fn draw_rounded_rect(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
    ctx.begin_path();
    ctx.move_to(x + r, y);
    ctx.line_to(x + w - r, y);
    let _ = ctx.arc_to(x + w, y, x + w, y + r, r);
    ctx.line_to(x + w, y + h - r);
    let _ = ctx.arc_to(x + w, y + h, x + w - r, y + h, r);
    ctx.line_to(x + r, y + h);
    let _ = ctx.arc_to(x, y + h, x, y + h - r, r);
    ctx.line_to(x, y + r);
    let _ = ctx.arc_to(x, y, x + r, y, r);
    ctx.close_path();
}

fn color_with_alpha(hex: &str, alpha: f64) -> String {
    if hex.len() < 7 {
        return format!("rgba(255,255,255,{})", alpha);
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
    format!("rgba({},{},{},{})", r, g, b, alpha)
}

fn draw_overlay(ctx: &CanvasRenderingContext2d, title: &str, sub: &str) {
    let cw = canvas_width().to_num::<f64>();
    let ch = canvas_height().to_num::<f64>();
    ctx.set_fill_style_str("rgba(0,0,0,0.6)");
    ctx.fill_rect(0.0, 0.0, cw, ch);
    ctx.set_text_align("center");
    ctx.set_shadow_color("rgba(0,212,255,0.8)");
    ctx.set_shadow_blur(20.0);
    ctx.set_fill_style_str("#00e5ff");
    ctx.set_font("bold 52px 'Segoe UI',Arial,sans-serif");
    let _ = ctx.fill_text(title, cw / 2.0, ch / 2.0 - 20.0);
    ctx.set_shadow_blur(0.0);
    ctx.set_shadow_color("transparent");
    ctx.set_fill_style_str("rgba(255,255,255,0.7)");
    ctx.set_font("18px 'Segoe UI',Arial,sans-serif");
    let _ = ctx.fill_text(sub, cw / 2.0, ch / 2.0 + 25.0);
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}

// ============================================================================
// Entry Point
// ============================================================================

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("game-canvas")
        .ok_or("no canvas")?
        .dyn_into()?;
    canvas.set_width(canvas_width().to_num::<f64>() as u32);
    canvas.set_height(canvas_height().to_num::<f64>() as u32);
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d")?.ok_or("no 2d ctx")?.dyn_into()?;
    let game = Rc::new(RefCell::new(Game::new()));
    let perf = window.performance().ok_or("no perf")?;
    game.borrow_mut().last_time = perf.now();

    // Mouse move
    {
        let g = game.clone();
        let el: web_sys::Element = canvas.clone().into();
        let cw = canvas_width();
        let cl = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MouseEvent| {
            let r = el.get_bounding_client_rect();
            let scale = cw / Fp::from_num(r.width());
            let mouse_x = Fp::from_num(e.client_x() as f64 - r.left()) * scale;
            g.borrow_mut().mouse_move(mouse_x);
        });
        canvas.add_event_listener_with_callback("mousemove", cl.as_ref().unchecked_ref())?;
        cl.forget();
    }

    // Click
    {
        let g = game.clone();
        let cl = Closure::<dyn FnMut(_)>::new(move |_: web_sys::MouseEvent| {
            g.borrow_mut().click();
        });
        canvas.add_event_listener_with_callback("click", cl.as_ref().unchecked_ref())?;
        cl.forget();
    }

    // Key down
    {
        let g = game.clone();
        let cl = Closure::<dyn FnMut(_)>::new(move |e: web_sys::KeyboardEvent| {
            e.prevent_default();
            g.borrow_mut().key_down(&e.key());
        });
        document.add_event_listener_with_callback("keydown", cl.as_ref().unchecked_ref())?;
        cl.forget();
    }

    // Key up
    {
        let g = game.clone();
        let cl = Closure::<dyn FnMut(_)>::new(move |e: web_sys::KeyboardEvent| {
            g.borrow_mut().key_up(&e.key());
        });
        document.add_event_listener_with_callback("keyup", cl.as_ref().unchecked_ref())?;
        cl.forget();
    }

    // Animation loop
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let f2 = f.clone();
    let _perf2 = perf.clone();
    *f2.borrow_mut() = Some(Closure::new(move |_ts: f64| {
        let now = perf.now();
        let dt_secs = ((now - game.borrow().last_time) / 1000.0).min(0.05);
        let dt = Fp::from_num(dt_secs);
        game.borrow_mut().last_time = now;
        game.borrow_mut().update(dt);
        game.borrow().draw(&ctx);
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(f2.borrow().as_ref().unwrap());

    Ok(())
}