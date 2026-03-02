use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const CANVAS_WIDTH: f64 = 800.0;
const CANVAS_HEIGHT: f64 = 600.0;
const PADDLE_WIDTH: f64 = 120.0;
const PADDLE_HEIGHT: f64 = 14.0;
const PADDLE_Y: f64 = CANVAS_HEIGHT - 40.0;
const PADDLE_SPEED: f64 = 600.0;
const BALL_RADIUS: f64 = 8.0;
const BALL_INITIAL_SPEED: f64 = 350.0;
const BALL_SPEED_INCREMENT: f64 = 8.0;
const BALL_MAX_SPEED: f64 = 700.0;
const BALL_TRAIL_LENGTH: usize = 12;
const BRICK_ROWS: usize = 6;
const BRICK_COLS: usize = 12;
const BRICK_WIDTH: f64 = 58.0;
const BRICK_HEIGHT: f64 = 22.0;
const BRICK_PADDING: f64 = 6.0;
const BRICK_OFFSET_TOP: f64 = 60.0;
const BRICK_OFFSET_LEFT: f64 = (CANVAS_WIDTH - (BRICK_COLS as f64 * (BRICK_WIDTH + BRICK_PADDING) - BRICK_PADDING)) / 2.0;
const MAX_LIVES: u32 = 3;
const MAX_PARTICLES: usize = 500;

const ROW_COLORS: [&str; 6] = ["#ff3366","#ff6633","#ffcc00","#33ff66","#33ccff","#9966ff"];
const ROW_GLOW: [&str; 6] = ["rgba(255,51,102,0.5)","rgba(255,102,51,0.5)","rgba(255,204,0,0.5)","rgba(51,255,102,0.5)","rgba(51,204,255,0.5)","rgba(153,102,255,0.5)"];

#[derive(Clone,Copy)] struct Vec2{x:f64,y:f64}
impl Vec2{fn new(x:f64,y:f64)->Self{Self{x,y}} fn normalize(&self)->Self{let l=(self.x*self.x+self.y*self.y).sqrt();if l==0.0{return *self}Self{x:self.x/l,y:self.y/l}}}
#[derive(Clone,Copy)] struct Brick{x:f64,y:f64,alive:bool,row:usize}
#[derive(Clone,Copy)] struct Particle{x:f64,y:f64,vx:f64,vy:f64,life:f64,max_life:f64,size:f64,ci:usize,active:bool}
impl Particle{fn new()->Self{Self{x:0.0,y:0.0,vx:0.0,vy:0.0,life:0.0,max_life:0.0,size:0.0,ci:0,active:false}}}
#[derive(Clone,Copy)] struct Trail{x:f64,y:f64,a:f64}
#[derive(PartialEq,Clone,Copy)] enum GS{Wait,Play,Over,Won}

struct Game{px:f64,bp:Vec2,bv:Vec2,bs:f64,bricks:Vec<Brick>,parts:Vec<Particle>,trail:Vec<Trail>,score:u32,lives:u32,state:GS,lp:bool,rp:bool,lt:f64,broken:u32,total:u32,fc:u64}

impl Game{
fn new()->Self{let mut g=Game{px:CANVAS_WIDTH/2.0-PADDLE_WIDTH/2.0,bp:Vec2::new(CANVAS_WIDTH/2.0,PADDLE_Y-BALL_RADIUS-2.0),bv:Vec2::new(0.0,0.0),bs:BALL_INITIAL_SPEED,bricks:Vec::new(),parts:vec![Particle::new();MAX_PARTICLES],trail:Vec::with_capacity(BALL_TRAIL_LENGTH),score:0,lives:MAX_LIVES,state:GS::Wait,lp:false,rp:false,lt:0.0,broken:0,total:0,fc:0};g.init_bricks();g}

fn init_bricks(&mut self){self.bricks.clear();for r in 0..BRICK_ROWS{for c in 0..BRICK_COLS{let x=BRICK_OFFSET_LEFT+c as f64*(BRICK_WIDTH+BRICK_PADDING);let y=BRICK_OFFSET_TOP+r as f64*(BRICK_HEIGHT+BRICK_PADDING);self.bricks.push(Brick{x,y,alive:true,row:r})}}self.total=(BRICK_ROWS*BRICK_COLS)as u32}

fn reset(&mut self){self.px=CANVAS_WIDTH/2.0-PADDLE_WIDTH/2.0;self.bp=Vec2::new(CANVAS_WIDTH/2.0,PADDLE_Y-BALL_RADIUS-2.0);self.bv=Vec2::new(0.0,0.0);self.bs=BALL_INITIAL_SPEED;self.score=0;self.lives=MAX_LIVES;self.broken=0;self.state=GS::Wait;self.trail.clear();self.init_bricks();for p in &mut self.parts{p.active=false}}

fn launch(&mut self){let a=-std::f64::consts::FRAC_PI_4+prand(self.fc)*std::f64::consts::FRAC_PI_2;self.bv=Vec2::new(a.sin(),-a.cos());self.state=GS::Play}

fn reset_ball(&mut self){self.bp=Vec2::new(self.px+PADDLE_WIDTH/2.0,PADDLE_Y-BALL_RADIUS-2.0);self.bv=Vec2::new(0.0,0.0);self.bs=(BALL_INITIAL_SPEED+self.broken as f64*(BALL_SPEED_INCREMENT*0.5)).min(BALL_MAX_SPEED);self.trail.clear();self.state=GS::Wait}

fn spawn_fx(&mut self,x:f64,y:f64,ci:usize,n:usize){let mut s=0;for p in &mut self.parts{if!p.active&&s<n{let a=prand(self.fc+s as u64*7)*std::f64::consts::TAU;let sp=50.0+prand(self.fc+s as u64*13)*200.0;p.x=x;p.y=y;p.vx=a.cos()*sp;p.vy=a.sin()*sp;p.life=0.5+prand(self.fc+s as u64*19)*0.7;p.max_life=p.life;p.size=2.0+prand(self.fc+s as u64*23)*4.0;p.ci=ci;p.active=true;s+=1}}}

fn update(&mut self,dt:f64){
self.fc+=1;
for p in &mut self.parts{if p.active{p.x+=p.vx*dt;p.y+=p.vy*dt;p.vy+=150.0*dt;p.life-=dt;if p.life<=0.0{p.active=false}}}
if self.state!=GS::Play{if self.state==GS::Wait{self.bp.x=self.px+PADDLE_WIDTH/2.0;self.bp.y=PADDLE_Y-BALL_RADIUS-2.0}return}
if self.lp{self.px-=PADDLE_SPEED*dt}
if self.rp{self.px+=PADDLE_SPEED*dt}
self.px=self.px.max(0.0).min(CANVAS_WIDTH-PADDLE_WIDTH);
self.trail.push(Trail{x:self.bp.x,y:self.bp.y,a:1.0});
if self.trail.len()>BALL_TRAIL_LENGTH{self.trail.remove(0);}
let tl=self.trail.len();for(i,t)in self.trail.iter_mut().enumerate(){t.a=(i as f64+1.0)/tl as f64*0.6}
self.bp.x+=self.bv.x*self.bs*dt;self.bp.y+=self.bv.y*self.bs*dt;
if self.bp.x-BALL_RADIUS<=0.0{self.bp.x=BALL_RADIUS;self.bv.x=self.bv.x.abs()}
if self.bp.x+BALL_RADIUS>=CANVAS_WIDTH{self.bp.x=CANVAS_WIDTH-BALL_RADIUS;self.bv.x=-self.bv.x.abs()}
if self.bp.y-BALL_RADIUS<=0.0{self.bp.y=BALL_RADIUS;self.bv.y=self.bv.y.abs()}
if self.bp.y+BALL_RADIUS>=CANVAS_HEIGHT{self.lives-=1;if self.lives==0{self.state=GS::Over}else{self.reset_ball()}return}
if self.bv.y>0.0{let pl=self.px;let pr=self.px+PADDLE_WIDTH;if self.bp.y+BALL_RADIUS>=PADDLE_Y&&self.bp.y+BALL_RADIUS<=PADDLE_Y+PADDLE_HEIGHT+4.0&&self.bp.x>=pl-BALL_RADIUS&&self.bp.x<=pr+BALL_RADIUS{let hp=(self.bp.x-pl)/PADDLE_WIDTH;let a=(hp-0.5)*std::f64::consts::FRAC_PI_3*2.5;self.bv=Vec2::new(a.sin(),-a.cos()).normalize();self.bp.y=PADDLE_Y-BALL_RADIUS}}
let mut hb=None;for(i,b)in self.bricks.iter().enumerate(){if!b.alive{continue}let cx=self.bp.x.max(b.x).min(b.x+BRICK_WIDTH);let cy=self.bp.y.max(b.y).min(b.y+BRICK_HEIGHT);let dx=self.bp.x-cx;let dy=self.bp.y-cy;if dx*dx+dy*dy<=BALL_RADIUS*BALL_RADIUS{hb=Some(i);let bcx=b.x+BRICK_WIDTH/2.0;let bcy=b.y+BRICK_HEIGHT/2.0;let dfx=self.bp.x-bcx;let dfy=self.bp.y-bcy;if dfx.abs()/BRICK_WIDTH>dfy.abs()/BRICK_HEIGHT{self.bv.x=if dfx>0.0{self.bv.x.abs()}else{-self.bv.x.abs()}}else{self.bv.y=if dfy>0.0{self.bv.y.abs()}else{-self.bv.y.abs()}}break}}
if let Some(i)=hb{let b=self.bricks[i];self.bricks[i].alive=false;self.score+=(BRICK_ROWS-b.row)as u32*10;self.broken+=1;self.bs=(self.bs+BALL_SPEED_INCREMENT).min(BALL_MAX_SPEED);self.spawn_fx(b.x+BRICK_WIDTH/2.0,b.y+BRICK_HEIGHT/2.0,b.row,15);if self.broken>=self.total{self.state=GS::Won}}
}

fn draw(&self,ctx:&CanvasRenderingContext2d){
ctx.set_fill_style_str("#0a0a2e");ctx.fill_rect(0.0,0.0,CANVAS_WIDTH,CANVAS_HEIGHT);
ctx.set_stroke_style_str("rgba(255,255,255,0.03)");ctx.set_line_width(1.0);
let mut gx=0.0;while gx<CANVAS_WIDTH{ctx.begin_path();ctx.move_to(gx,0.0);ctx.line_to(gx,CANVAS_HEIGHT);ctx.stroke();gx+=40.0}
let mut gy=0.0;while gy<CANVAS_HEIGHT{ctx.begin_path();ctx.move_to(0.0,gy);ctx.line_to(CANVAS_WIDTH,gy);ctx.stroke();gy+=40.0}
for b in &self.bricks{if!b.alive{continue}let c=ROW_COLORS[b.row%6];let g=ROW_GLOW[b.row%6];ctx.set_shadow_color(g);ctx.set_shadow_blur(12.0);ctx.set_fill_style_str(c);rr(ctx,b.x,b.y,BRICK_WIDTH,BRICK_HEIGHT,4.0);ctx.fill();ctx.set_shadow_blur(0.0);ctx.set_fill_style_str("rgba(255,255,255,0.25)");rr(ctx,b.x+2.0,b.y+2.0,BRICK_WIDTH-4.0,BRICK_HEIGHT/3.0,2.0);ctx.fill()}
ctx.set_shadow_blur(0.0);ctx.set_shadow_color("transparent");
for p in &self.parts{if!p.active{continue}let a=(p.life/p.max_life).max(0.0);let c=ROW_COLORS[p.ci%6];ctx.set_fill_style_str(&ca(c,a));ctx.begin_path();let _=ctx.arc(p.x,p.y,p.size*a,0.0,std::f64::consts::TAU);ctx.fill()}
for t in &self.trail{ctx.set_fill_style_str(&format!("rgba(100,200,255,{})",t.a*0.4));ctx.begin_path();let _=ctx.arc(t.x,t.y,BALL_RADIUS*t.a,0.0,std::f64::consts::TAU);ctx.fill()}
ctx.set_shadow_color("rgba(100,200,255,0.8)");ctx.set_shadow_blur(20.0);ctx.set_fill_style_str("#ffffff");ctx.begin_path();let _=ctx.arc(self.bp.x,self.bp.y,BALL_RADIUS,0.0,std::f64::consts::TAU);ctx.fill();ctx.set_shadow_blur(0.0);ctx.set_shadow_color("transparent");
ctx.set_shadow_color("rgba(0,212,255,0.6)");ctx.set_shadow_blur(15.0);
{let gr=ctx.create_linear_gradient(self.px,PADDLE_Y,self.px,PADDLE_Y+PADDLE_HEIGHT);let _=gr.add_color_stop(0.0,"#00e5ff");let _=gr.add_color_stop(1.0,"#0088aa");ctx.set_fill_style_canvas_gradient(&gr)}
rr(ctx,self.px,PADDLE_Y,PADDLE_WIDTH,PADDLE_HEIGHT,7.0);ctx.fill();
ctx.set_shadow_blur(0.0);ctx.set_shadow_color("transparent");ctx.set_fill_style_str("rgba(255,255,255,0.3)");rr(ctx,self.px+4.0,PADDLE_Y+2.0,PADDLE_WIDTH-8.0,PADDLE_HEIGHT/3.0,4.0);ctx.fill();
// HUD
ctx.set_fill_style_str("rgba(255,255,255,0.9)");ctx.set_font("bold 18px 'Segoe UI',Arial,sans-serif");ctx.set_text_align("left");let _=ctx.fill_text(&format!("SCORE: {}",self.score),20.0,30.0);ctx.set_text_align("right");let _=ctx.fill_text(&format!("LIVES: {}",self.lives),CANVAS_WIDTH-20.0,30.0);ctx.set_text_align("center");ctx.set_fill_style_str("rgba(255,255,255,0.3)");ctx.set_font("12px 'Segoe UI',Arial,sans-serif");let sp=((self.bs-BALL_INITIAL_SPEED)/(BALL_MAX_SPEED-BALL_INITIAL_SPEED)*100.0)as u32;let _=ctx.fill_text(&format!("SPEED +{}%",sp),CANVAS_WIDTH/2.0,30.0);ctx.set_stroke_style_str("rgba(0,212,255,0.3)");ctx.set_line_width(1.0);ctx.begin_path();ctx.move_to(0.0,44.0);ctx.line_to(CANVAS_WIDTH,44.0);ctx.stroke();
// Overlays
match self.state{
GS::Wait=>{let(t,s)=if self.lives==MAX_LIVES&&self.score==0{("BREAKOUT","Click or press Space to start")}else{("READY","Click or press Space to launch")};overlay(ctx,t,s)}
GS::Over=>{overlay(ctx,"GAME OVER",&format!("Score: {} \u{2014} Press Space to restart",self.score))}
GS::Won=>{overlay(ctx,"YOU WIN!",&format!("Score: {} \u{2014} Press Space to play again",self.score))}
GS::Play=>{}}
}

fn mouse_move(&mut self,x:f64){if self.state==GS::Play||self.state==GS::Wait{self.px=(x-PADDLE_WIDTH/2.0).max(0.0).min(CANVAS_WIDTH-PADDLE_WIDTH)}}
fn click(&mut self){match self.state{GS::Wait=>self.launch(),GS::Over|GS::Won=>self.reset(),_=>{}}}
fn kd(&mut self,k:&str){match k{"ArrowLeft"|"a"|"A"=>self.lp=true,"ArrowRight"|"d"|"D"=>self.rp=true," "=>self.click(),_=>{}}}
fn ku(&mut self,k:&str){match k{"ArrowLeft"|"a"|"A"=>self.lp=false,"ArrowRight"|"d"|"D"=>self.rp=false,_=>{}}}
}

fn prand(s:u64)->f64{let mut v=s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);v^=v>>22;v^=v<<13;v^=v>>8;(v%10000)as f64/10000.0}
fn rr(ctx:&CanvasRenderingContext2d,x:f64,y:f64,w:f64,h:f64,r:f64){ctx.begin_path();ctx.move_to(x+r,y);ctx.line_to(x+w-r,y);let _=ctx.arc_to(x+w,y,x+w,y+r,r);ctx.line_to(x+w,y+h-r);let _=ctx.arc_to(x+w,y+h,x+w-r,y+h,r);ctx.line_to(x+r,y+h);let _=ctx.arc_to(x,y+h,x,y+h-r,r);ctx.line_to(x,y+r);let _=ctx.arc_to(x,y,x+r,y,r);ctx.close_path()}
fn ca(hex:&str,alpha:f64)->String{if hex.len()<7{return format!("rgba(255,255,255,{})",alpha)}let r=u8::from_str_radix(&hex[1..3],16).unwrap_or(255);let g=u8::from_str_radix(&hex[3..5],16).unwrap_or(255);let b=u8::from_str_radix(&hex[5..7],16).unwrap_or(255);format!("rgba({},{},{},{})",r,g,b,alpha)}
fn overlay(ctx:&CanvasRenderingContext2d,title:&str,sub:&str){ctx.set_fill_style_str("rgba(0,0,0,0.6)");ctx.fill_rect(0.0,0.0,CANVAS_WIDTH,CANVAS_HEIGHT);ctx.set_text_align("center");ctx.set_shadow_color("rgba(0,212,255,0.8)");ctx.set_shadow_blur(20.0);ctx.set_fill_style_str("#00e5ff");ctx.set_font("bold 52px 'Segoe UI',Arial,sans-serif");let _=ctx.fill_text(title,CANVAS_WIDTH/2.0,CANVAS_HEIGHT/2.0-20.0);ctx.set_shadow_blur(0.0);ctx.set_shadow_color("transparent");ctx.set_fill_style_str("rgba(255,255,255,0.7)");ctx.set_font("18px 'Segoe UI',Arial,sans-serif");let _=ctx.fill_text(sub,CANVAS_WIDTH/2.0,CANVAS_HEIGHT/2.0+25.0);}
fn raf(f:&Closure<dyn FnMut(f64)>){web_sys::window().unwrap().request_animation_frame(f.as_ref().unchecked_ref()).unwrap();}

#[wasm_bindgen(start)]
pub fn start()->Result<(),JsValue>{
let w=web_sys::window().ok_or("no window")?;
let doc=w.document().ok_or("no document")?;
let cvs:HtmlCanvasElement=doc.get_element_by_id("game-canvas").ok_or("no canvas")?.dyn_into()?;
cvs.set_width(CANVAS_WIDTH as u32);cvs.set_height(CANVAS_HEIGHT as u32);
let ctx:CanvasRenderingContext2d=cvs.get_context("2d")?.ok_or("no 2d ctx")?.dyn_into()?;
let game=Rc::new(RefCell::new(Game::new()));
let perf=w.performance().ok_or("no perf")?;
game.borrow_mut().lt=perf.now();
{let g=game.clone();let c:web_sys::Element=cvs.clone().into();let cl=Closure::<dyn FnMut(_)>::new(move|e:web_sys::MouseEvent|{let r=c.get_bounding_client_rect();let sx=CANVAS_WIDTH/r.width();g.borrow_mut().mouse_move((e.client_x()as f64-r.left())*sx)});cvs.add_event_listener_with_callback("mousemove",cl.as_ref().unchecked_ref())?;cl.forget()}
{let g=game.clone();let cl=Closure::<dyn FnMut(_)>::new(move|_:web_sys::MouseEvent|{g.borrow_mut().click()});cvs.add_event_listener_with_callback("click",cl.as_ref().unchecked_ref())?;cl.forget()}
{let g=game.clone();let cl=Closure::<dyn FnMut(_)>::new(move|e:web_sys::KeyboardEvent|{e.prevent_default();g.borrow_mut().kd(&e.key())});doc.add_event_listener_with_callback("keydown",cl.as_ref().unchecked_ref())?;cl.forget()}
{let g=game.clone();let cl=Closure::<dyn FnMut(_)>::new(move|e:web_sys::KeyboardEvent|{g.borrow_mut().ku(&e.key())});doc.add_event_listener_with_callback("keyup",cl.as_ref().unchecked_ref())?;cl.forget()}
let f:Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>=Rc::new(RefCell::new(None));
let f2=f.clone();
let perf2=perf.clone();
*f2.borrow_mut()=Some(Closure::new(move|_ts:f64|{let now=perf.now();let dt=((now-game.borrow().lt)/1000.0).min(0.05);game.borrow_mut().lt=now;game.borrow_mut().update(dt);game.borrow().draw(&ctx);raf(f.borrow().as_ref().unwrap())}));
raf(f2.borrow().as_ref().unwrap());
Ok(())
}
