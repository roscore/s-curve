//! 3D scene: a revolute joint with a link (rotary mode) or a carriage on a belt-driven rail
//! (linear mode), with start / target ghosts, velocity indicator and torque tint.
//! Orbit / pan / zoom camera with inertia and direct manipulation of the target ghost.
use crate::gl::{self, Mesh, Paint, Renderer};
use crate::math::{self, Orbit, OrbitCam, M4, V};
use web_sys::HtmlCanvasElement;

/// What to show this frame (positions in rad or m).
#[derive(Clone, Copy, Default)]
pub struct Scene {
    pub linear: bool,
    pub d: f64,
    pub p: f64,
    pub v: f64,
    /// |τ_m| / peak, 0-1.
    pub tint: f32,
    /// Second axis: distance, position, velocity, tint.
    pub axis2: Option<(f64, f64, f64, f32)>,
}

#[derive(Clone, Copy)]
pub struct Toggles {
    pub ghosts: bool,
    pub vel: bool,
    pub tint: bool,
    pub axis2: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Preset {
    Persp,
    Top,
    Side,
    Front,
}

const FOV: f32 = 0.8;
const HUB_Y: f32 = 0.53;
const LINK: f32 = 0.62;
const CAR_Y: f32 = 0.085;
const AXIS2_Z: [f32; 2] = [1.5, 0.35];
const UP: V = [0.0, 1.0, 0.0];

pub struct View {
    r: Renderer,
    cam: OrbitCam,
    canvas: HtmlCanvasElement,
    cyl: Mesh,
    cube: Mesh,
    ball: Mesh,
    link: Mesh,
    pub toggles: Toggles,
    pub hover: bool,
    dragging: bool,
    grab: f64,
    preset: Preset,
    inv_vp: M4,
    /// Active pointers (id, x, y in CSS px) and the button of a single-pointer drag.
    ptrs: Vec<(i32, f32, f32)>,
    button: i16,
}

fn cyl_at(c: V, r: f32, h: f32) -> M4 {
    math::basis([1., 0., 0.], UP, [0., 0., 1.], [2.0 * r, h, 2.0 * r], c)
}
fn box_at(c: V, s: V) -> M4 {
    math::basis([1., 0., 0.], UP, [0., 0., 1.], s, c)
}
fn ball_at(c: V, r: f32) -> M4 {
    box_at(c, [r; 3])
}
fn dir(th: f64) -> V {
    let (s, c) = (th as f32).sin_cos();
    [c, 0.0, -s]
}
fn link_at(th: f64, z: f32) -> M4 {
    let d = dir(th);
    math::basis(UP, d, math::cross(UP, d), [0.04; 3], [d[0] * LINK / 2.0, HUB_Y, z + d[2] * LINK / 2.0])
}
fn wrap_pi(x: f64) -> f64 {
    (x + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
fn mix(a: [f32; 3], b: [f32; 3], t: f32, alpha: f32) -> [f32; 4] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, alpha]
}
/// Rail extent (x0, x1) for a linear move of distance d.
fn rail(d: f64) -> (f32, f32) {
    let d = d as f32;
    (d.min(0.0) - 0.12, d.max(0.0).max(0.3) + 0.12)
}

/// Draw list entry: mesh, model, paint, casts a shadow.
type Item<'a> = (&'a Mesh, M4, Paint, bool);

impl View {
    pub fn new(canvas: &HtmlCanvasElement, dark: bool) -> Option<View> {
        let mut r = Renderer::new(canvas)?;
        r.theme = if dark { gl::DARK } else { gl::LIGHT };
        r.cell = 0.25;
        r.fog = [6.0, 30.0];
        let o = Orbit { target: [0.0, 0.3, 0.0], yaw: 0.7, pitch: 0.5, dist: 2.6 };
        Some(View {
            cyl: r.mesh(&gl::cylinder(32))?,
            cube: r.mesh(&gl::cube())?,
            ball: r.mesh(&gl::sphere(20))?,
            link: r.mesh(&gl::capsule(16, LINK / 2.0 / 0.04 - 1.0))?,
            r,
            cam: OrbitCam::new(o),
            canvas: canvas.clone(),
            toggles: Toggles { ghosts: true, vel: true, tint: true, axis2: true },
            hover: false,
            dragging: false,
            grab: 0.0,
            preset: Preset::Persp,
            inv_vp: math::IDENTITY,
            ptrs: vec![],
            button: 0,
        })
    }

    /// Glide to a view preset framing the current scene.
    pub fn set_preset(&mut self, p: Preset, sc: &Scene) {
        self.preset = p;
        let two = sc.axis2.is_some() && self.toggles.axis2;
        let (target, size) = if sc.linear {
            let (x0, x1) = rail(sc.d);
            let z = if two { AXIS2_Z[1] / 2.0 } else { 0.0 };
            ([(x0 + x1) / 2.0, 0.05, z], (x1 - x0).max(if two { 0.6 } else { 0.0 }))
        } else {
            let z = if two { AXIS2_Z[0] / 2.0 } else { 0.0 };
            ([0.0, 0.3, z], if two { 2.6 } else { 1.6 })
        };
        // Fit the width too on portrait screens.
        let aspect = self.canvas.client_width().max(1) as f32 / self.canvas.client_height().max(1) as f32;
        let dist = size * 0.75 / (FOV / 2.0).tan() / aspect.clamp(0.4, 1.0);
        let (yaw, pitch) = match p {
            Preset::Persp => (0.7, 0.5),
            Preset::Top => (0.0, math::PITCH_MAX),
            Preset::Side => (std::f32::consts::FRAC_PI_2, 0.05),
            Preset::Front => (0.0, 0.05),
        };
        self.cam.set_goal(Orbit { target, yaw, pitch, dist });
    }
    pub fn set_dark(&mut self, dark: bool) {
        self.r.theme = if dark { gl::DARK } else { gl::LIGHT };
    }
    pub fn reset(&mut self, sc: &Scene) {
        self.set_preset(self.preset, sc);
    }

    fn ray(&self, x: f32, y: f32) -> (V, V) {
        let (w, h) = (self.canvas.client_width().max(1) as f32, self.canvas.client_height().max(1) as f32);
        math::ndc_ray(&self.inv_vp, x / w * 2.0 - 1.0, 1.0 - y / h * 2.0)
    }

    /// Is the screen point over the draggable target ghost?
    fn over_target(&self, x: f32, y: f32, sc: &Scene) -> bool {
        if !self.toggles.ghosts {
            return false;
        }
        let (o, d) = self.ray(x, y);
        let (c, r) = if sc.linear {
            ([sc.d as f32, CAR_Y, 0.0], 0.075)
        } else {
            let e = dir(sc.d);
            ([e[0] * LINK, HUB_Y, e[2] * LINK], 0.1)
        };
        math::ray_sphere(o, d, c, r).is_some()
    }

    /// New distance D under the pointer while dragging the target ghost.
    fn drag_value(&self, x: f32, y: f32, sc: &Scene) -> Option<f64> {
        let (o, d) = self.ray(x, y);
        if sc.linear {
            let n = if self.cam.cur.pitch > 0.35 { UP } else { [0.0, 0.0, 1.0] };
            let hit = math::ray_plane(o, d, [0.0, CAR_Y, 0.0], n)?;
            Some(((hit[0] as f64 + self.grab) * 1000.0).round() / 1000.0)
        } else {
            let hit = math::ray_plane(o, d, [0.0, HUB_Y, 0.0], UP)?;
            if hit[0].hypot(hit[2]) < 0.05 {
                return None;
            }
            let th = (-hit[2] as f64).atan2(hit[0] as f64);
            Some(((sc.d + wrap_pi(th - sc.d)) * 1000.0).round() / 1000.0)
        }
    }

    /// Pointer pressed. Returns true when it grabbed the target ghost.
    pub fn down(&mut self, id: i32, x: f32, y: f32, button: i16, sc: &Scene) -> bool {
        self.ptrs.retain(|p| p.0 != id);
        self.ptrs.push((id, x, y));
        self.button = button;
        self.cam.held = true;
        if self.ptrs.len() == 1 && button == 0 && self.over_target(x, y, sc) {
            self.dragging = true;
            self.grab = 0.0;
            if sc.linear {
                self.grab = sc.d - self.drag_value(x, y, sc).unwrap_or(sc.d);
            }
            return true;
        }
        self.dragging = false;
        false
    }

    /// Pointer moved. Returns a new distance D while dragging the target ghost.
    pub fn moved(&mut self, id: i32, x: f32, y: f32, sc: &Scene) -> Option<f64> {
        let Some(i) = self.ptrs.iter().position(|p| p.0 == id) else {
            self.hover = self.over_target(x, y, sc);
            return None;
        };
        let (_, px, py) = self.ptrs[i];
        if self.ptrs.len() == 1 {
            self.ptrs[i] = (id, x, y);
            let (dx, dy) = (x - px, y - py);
            if self.dragging {
                return self.drag_value(x, y, sc);
            }
            if self.button == 2 || self.button == 1 {
                self.pan(dx, dy);
            } else {
                self.cam.rotate(-dx * 0.008, dy * 0.008);
            }
            return None;
        }
        // Two fingers: pan by the midpoint, zoom by the spread.
        let o = self.ptrs[1 - i.min(1)];
        let (mx0, my0, d0) = ((px + o.1) / 2.0, (py + o.2) / 2.0, (px - o.1).hypot(py - o.2));
        self.ptrs[i] = (id, x, y);
        let (mx1, my1, d1) = ((x + o.1) / 2.0, (y + o.2) / 2.0, (x - o.1).hypot(y - o.2));
        self.pan(mx1 - mx0, my1 - my0);
        if d0 > 1.0 && d1 > 1.0 {
            self.cam.zoom(d0 / d1);
        }
        None
    }

    pub fn up(&mut self, id: i32) {
        self.ptrs.retain(|p| p.0 != id);
        if self.ptrs.is_empty() {
            self.cam.held = false;
            self.dragging = false;
        }
    }

    pub fn wheel(&mut self, dy: f32) {
        self.cam.zoom((dy * 0.0015).exp());
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    fn pan(&mut self, dx: f32, dy: f32) {
        let o = self.cam.cur;
        let f = math::norm(math::sub(o.target, o.eye()));
        let right = math::norm(math::cross(f, UP));
        let up = math::cross(right, f);
        let k = o.dist * 2.0 * (FOV / 2.0).tan() / self.canvas.client_height().max(1) as f32;
        self.cam.pan(math::add(math::scale(right, -dx * k), math::scale(up, dy * k)));
    }

    /// Advance the camera and draw one frame.
    pub fn frame(&mut self, sc: &Scene, dt: f32) {
        let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0).min(2.0);
        let (cw, ch) = (self.canvas.client_width(), self.canvas.client_height());
        let (w, h) = (((cw as f64) * dpr) as u32, ((ch as f64) * dpr) as u32);
        if w == 0 || h == 0 {
            return;
        }
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        self.cam.step(dt);
        let o = self.cam.cur;
        let eye = o.eye();
        let view = math::look_at(eye, o.target, UP);
        let proj = math::perspective(FOV, w as f32 / h as f32, 0.02, 200.0);
        self.inv_vp = math::invert(&math::mul(&proj, &view)).unwrap_or(math::IDENTITY);
        self.r.begin(&view, &proj, eye, [0, 0, w as i32, h as i32]);

        let mut items: Vec<Item> = vec![];
        let mut ghosts: Vec<Item> = vec![];
        let mut segs: Vec<(V, V, [f32; 4])> = vec![];
        self.axis(sc, sc.d, sc.p, sc.v, sc.tint, 0.0, true, &mut items, &mut ghosts, &mut segs);
        if let (Some((d2, p2, v2, t2)), true) = (sc.axis2, self.toggles.axis2) {
            let z = AXIS2_Z[sc.linear as usize];
            self.axis(sc, d2, p2, v2, t2, z, false, &mut items, &mut ghosts, &mut segs);
        }
        for it in items.iter().filter(|i| i.3) {
            self.r.shadow(it.0, &it.1);
        }
        for it in items.iter().chain(&ghosts) {
            self.r.draw(it.0, &it.1, it.2);
        }
        // Lines grouped by colour, uploaded per frame.
        let mut colors: Vec<[f32; 4]> = segs.iter().map(|s| s.2).collect();
        colors.dedup();
        for c in colors {
            let pts: Vec<V> = segs.iter().filter(|s| s.2 == c).flat_map(|s| [s.0, s.1]).collect();
            if let Some(m) = self.r.mesh(&gl::lines(&pts)) {
                self.r.draw(&m, &math::IDENTITY, Paint::flat(c));
                self.r.free(m);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn axis<'a>(
        &'a self,
        sc: &Scene,
        d: f64,
        p: f64,
        v: f64,
        tint: f32,
        z: f32,
        primary: bool,
        items: &mut Vec<Item<'a>>,
        ghosts: &mut Vec<Item<'a>>,
        segs: &mut Vec<(V, V, [f32; 4])>,
    ) {
        let t = self.toggles;
        let neutral = [0.55, 0.57, 0.62];
        let housing = Paint::lit(mix(neutral, [0.92, 0.12, 0.08], if t.tint { tint.clamp(0.0, 1.0) } else { 0.0 }, 1.0));
        let metal = Paint::lit([0.32, 0.34, 0.38, 1.0]);
        let accent = Paint::lit(if primary { [0.15, 0.55, 0.95, 1.0] } else { [0.95, 0.6, 0.15, 1.0] });
        let ghost_c = |a: f32| [0.6, 0.85, 1.0, a];
        let mut target = Paint::lit(ghost_c(0.35));
        if primary && (self.hover || self.dragging) {
            target.glow = 0.45;
            target.color[3] = 0.55;
        }
        let start = Paint::lit([0.8, 0.8, 0.8, 0.22]);
        let vel_c = [1.0, 0.85, 0.2, 1.0];
        let sweep_c = if primary { [0.35, 0.75, 1.0, 0.9] } else { [1.0, 0.7, 0.3, 0.9] };
        if sc.linear {
            let (x0, x1) = rail(d);
            let (cx, len) = ((x0 + x1) / 2.0, x1 - x0);
            items.push((&self.cube, box_at([cx, 0.015, z], [len + 0.04, 0.03, 0.09]), metal, true));
            items.push((&self.cube, box_at([cx, 0.045, z], [len, 0.03, 0.03]), Paint::lit([0.7, 0.72, 0.75, 1.0]), true));
            let car = |x: f64| box_at([x as f32, CAR_Y, z], [0.09, 0.05, 0.08]);
            items.push((&self.cube, car(p), accent, true));
            let motor = math::basis([1., 0., 0.], [0., 0., 1.], [0., -1., 0.], [0.09, 0.08, 0.09], [x0 - 0.05, 0.06, z + 0.0]);
            items.push((&self.cyl, motor, housing, true));
            // Belt loop beside the rail; teeth ticks travel with the carriage (top run) and back (bottom).
            let zb = z - 0.06;
            let belt_c = [0.1, 0.1, 0.1, 1.0];
            let (yt, yb) = (0.105, 0.06);
            segs.push(([x0, yt, zb], [x1, yt, zb], belt_c));
            segs.push(([x0, yb, zb], [x1, yb, zb], belt_c));
            segs.push(([x0, yt, zb], [x0 - 0.02, (yt + yb) / 2.0, zb], belt_c));
            segs.push(([x0 - 0.02, (yt + yb) / 2.0, zb], [x0, yb, zb], belt_c));
            segs.push(([x1, yt, zb], [x1 + 0.02, (yt + yb) / 2.0, zb], belt_c));
            segs.push(([x1 + 0.02, (yt + yb) / 2.0, zb], [x1, yb, zb], belt_c));
            let sp = 0.02f32;
            for k in 0..(len / sp) as i32 {
                for (y, sgn) in [(yt, 1.0), (yb, -1.0)] {
                    let x = x0 + (k as f32 * sp + sgn * p as f32).rem_euclid(len);
                    segs.push(([x, y, zb], [x, y + 0.008, zb], [0.45, 0.45, 0.45, 1.0]));
                }
            }
            if t.ghosts {
                ghosts.push((&self.cube, car(0.0), start, false));
                ghosts.push((&self.cube, car(d), if primary { target } else { Paint::lit(ghost_c(0.25)) }, false));
            }
            segs.push(([0.0, 0.002, z + 0.05], [d as f32, 0.002, z + 0.05], sweep_c));
            if t.vel && v.abs() > 1e-9 {
                let (a, b) = ([p as f32, 0.14, z], [(p + v * 0.4) as f32, 0.14, z]);
                let s = (v.signum() as f32) * 0.015;
                segs.push((a, b, vel_c));
                segs.push((b, [b[0] - s, 0.15, z], vel_c));
                segs.push((b, [b[0] - s, 0.13, z], vel_c));
            }
        } else {
            items.push((&self.cyl, cyl_at([0.0, 0.15, z], 0.09, 0.3), metal, true));
            items.push((&self.cyl, cyl_at([0.0, 0.38, z], 0.11, 0.16), housing, true));
            items.push((&self.cyl, cyl_at([0.0, 0.485, z], 0.135, 0.05), Paint::lit([0.22, 0.23, 0.26, 1.0]), true));
            items.push((&self.cyl, cyl_at([0.0, HUB_Y, z], 0.06, 0.05), metal, true));
            items.push((&self.link, link_at(p, z), accent, true));
            let e = |th: f64, r: f32| {
                let q = dir(th);
                [q[0] * r, HUB_Y, z + q[2] * r]
            };
            items.push((&self.ball, ball_at(e(p, LINK), 0.055), Paint::lit([0.9, 0.9, 0.92, 1.0]), true));
            if t.ghosts {
                ghosts.push((&self.link, link_at(0.0, z), start, false));
                ghosts.push((&self.ball, ball_at(e(0.0, LINK), 0.055), start, false));
                let tg = if primary { target } else { Paint::lit(ghost_c(0.25)) };
                ghosts.push((&self.link, link_at(d, z), tg, false));
                ghosts.push((&self.ball, ball_at(e(d, LINK), 0.07), tg, false));
            }
            // Swept angle on the floor.
            let n = ((d.abs() * 12.0) as usize).clamp(2, 400);
            let fl = |th: f64, r: f32| {
                let q = dir(th);
                [q[0] * r, 0.003, z + q[2] * r]
            };
            for i in 0..n {
                let (a, b) = (d * i as f64 / n as f64, d * (i + 1) as f64 / n as f64);
                segs.push((fl(a, 0.7), fl(b, 0.7), sweep_c));
            }
            segs.push((fl(0.0, 0.15), fl(0.0, 0.8), sweep_c));
            segs.push((fl(d, 0.15), fl(d, 0.8), sweep_c));
            if t.vel && v.abs() > 1e-9 {
                let sw = (v * 0.35).clamp(-4.5, 4.5);
                let n = 24;
                let r = LINK + 0.13;
                for i in 0..n {
                    segs.push((e(p + sw * i as f64 / n as f64, r), e(p + sw * (i + 1) as f64 / n as f64, r), vel_c));
                }
                let tip = e(p + sw, r);
                let back = p + sw - sw.signum() * 0.12;
                for rr in [r - 0.04, r + 0.04] {
                    segs.push((tip, e(back, rr), vel_c));
                }
            }
        }
    }
}
