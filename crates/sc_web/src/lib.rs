//! Browser UI: inputs, results, playback, 3D view and 2D plots. Plain web-sys, no framework.
pub mod gl;
pub mod i18n;
pub mod math;
pub mod plot;
pub mod view3d;

use i18n::{tr, Lang};
use sc_core::{load_stats, sync, time_penalty, Kind, Limits, Load, LoadStats, Profile, Synced};
use std::cell::RefCell;
use std::rc::Rc;
use view3d::{Preset, Scene, View};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, HtmlCanvasElement, HtmlElement, HtmlInputElement, HtmlSelectElement, PointerEvent, WheelEvent};

#[derive(Clone, Copy)]
struct Inputs {
    linear: bool,
    kind: Kind,
    d: f64,
    v: f64,
    a: f64,
    j: f64,
    inertia: f64,
    load: f64,
    ratio: f64,
    eff: f64,
    rated: f64,
    sync: bool,
    d2: f64,
}

const KNEE: Inputs = Inputs {
    linear: false,
    kind: Kind::SCurve,
    d: 1.5,
    v: 6.0,
    a: 40.0,
    j: 800.0,
    inertia: 0.15,
    load: 15.0,
    ratio: 9.0,
    eff: 0.85,
    rated: 3.0,
    sync: false,
    d2: 0.6,
};

const GANTRY: Inputs = Inputs {
    linear: true,
    kind: Kind::SCurve,
    d: 0.2,
    v: 0.3,
    a: 3.0,
    j: 100.0,
    inertia: 0.8,
    load: 2.0,
    ratio: 157.0,
    eff: 0.9,
    rated: 0.3,
    sync: false,
    d2: 0.12,
};

const NUM_IDS: [&str; 10] = ["d", "v", "a", "j", "inertia", "load", "ratio", "eff", "rated", "d2"];

struct Model {
    s: Profile,
    t: Profile,
    axis2: Option<Synced>,
    load: Load,
    ls: LoadStats,
    lt: LoadStats,
    traces: Vec<plot::Trace>,
}

impl Model {
    fn active(&self, kind: Kind) -> (&Profile, &LoadStats) {
        if kind == Kind::SCurve { (&self.s, &self.ls) } else { (&self.t, &self.lt) }
    }
    /// Playback length: the animated profile, or axis 2 if it is longer (bottleneck).
    fn t_end(&self, kind: Kind) -> f64 {
        let a2 = self.axis2.as_ref().map_or(0.0, |s| s.profile.total());
        self.active(kind).0.total().max(a2)
    }
    fn span(&self) -> f64 {
        let a2 = self.axis2.as_ref().map_or(0.0, |s| s.profile.total());
        self.s.total().max(self.t.total()).max(a2)
    }
}

struct App {
    lang: Lang,
    inp: Inputs,
    m: Model,
    err: bool,
    play: bool,
    looping: bool,
    speed: f64,
    t: f64,
    last: f64,
    view: Option<View>,
    dark: bool,
    hud: String,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.borrow_mut().as_mut().map(f))
}

fn doc() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
fn el(id: &str) -> Element {
    doc().get_element_by_id(id).unwrap_or_else(|| panic!("missing #{id}"))
}
fn input(id: &str) -> HtmlInputElement {
    el(id).unchecked_into()
}
fn select(id: &str) -> HtmlSelectElement {
    el(id).unchecked_into()
}
fn on(target: &web_sys::EventTarget, ev: &str, f: impl FnMut(Event) + 'static) {
    let c = Closure::<dyn FnMut(Event)>::new(f);
    let o = web_sys::AddEventListenerOptions::new();
    o.set_passive(false);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(ev, c.as_ref().unchecked_ref(), &o);
    c.forget();
}
fn on_id(id: &str, ev: &str, f: impl FnMut(Event) + 'static) {
    on(&el(id), ev, f);
}

/// Format with about four significant digits.
fn num(x: f64) -> String {
    let a = x.abs();
    if !x.is_finite() {
        "—".into()
    } else if a >= 1000.0 || a == 0.0 {
        format!("{x:.0}")
    } else if a >= 100.0 {
        format!("{x:.1}")
    } else if a >= 10.0 {
        format!("{x:.2}")
    } else if a >= 1.0 {
        format!("{x:.3}")
    } else if a >= 0.01 {
        format!("{x:.4}")
    } else {
        format!("{x:.2e}")
    }
}

fn unit(key: &str, linear: bool) -> &'static str {
    let (r, l) = match key {
        "pos" => ("rad", "m"),
        "vel" => ("rad/s", "m/s"),
        "acc" => ("rad/s²", "m/s²"),
        "jerk" => ("rad/s³", "m/s³"),
        "inertia" => ("kg·m²", "kg"),
        "load" => ("N·m", "N"),
        "ratio" => ("", "rad/m"),
        _ => ("", ""),
    };
    if linear { l } else { r }
}

fn read_inputs() -> Inputs {
    let n = |id: &str| input(id).value_as_number();
    Inputs {
        linear: select("mode").value() == "lin",
        kind: if select("kind").value() == "trap" { Kind::Trapezoid } else { Kind::SCurve },
        d: n("d"),
        v: n("v"),
        a: n("a"),
        j: n("j"),
        inertia: n("inertia"),
        load: n("load"),
        ratio: n("ratio"),
        eff: n("eff"),
        rated: n("rated"),
        sync: input("sync").checked(),
        d2: n("d2"),
    }
}

fn write_inputs(i: &Inputs) {
    select("mode").set_value(if i.linear { "lin" } else { "rot" });
    select("kind").set_value(if i.kind == Kind::Trapezoid { "trap" } else { "s" });
    let vals = [i.d, i.v, i.a, i.j, i.inertia, i.load, i.ratio, i.eff, i.rated, i.d2];
    for (id, v) in NUM_IDS.iter().zip(vals) {
        input(id).set_value(&format!("{v}"));
    }
    input("sync").set_checked(i.sync);
}

fn compute(i: &Inputs) -> Option<Model> {
    let lim = Limits { v: i.v, a: i.a, j: i.j };
    let load = Load { inertia: i.inertia, load: i.load, ratio: i.ratio, eff: i.eff };
    let ok = [i.inertia, i.load, i.rated].iter().all(|x| x.is_finite()) && i.inertia >= 0.0 && i.ratio > 0.0 && i.eff > 0.0 && i.eff <= 1.0;
    if !ok || !i.ratio.is_finite() {
        return None;
    }
    let s = Profile::plan(Kind::SCurve, i.d, lim).ok()?;
    let t = Profile::plan(Kind::Trapezoid, i.d, lim).ok()?;
    let target = if i.kind == Kind::SCurve { s.total() } else { t.total() };
    let axis2 = if i.sync { Some(sync(i.kind, i.d2, lim, target).ok()?) } else { None };
    let (ls, lt) = (load_stats(&s, &load), load_stats(&t, &load));
    let mut m = Model { s, t, axis2, load, ls, lt, traces: vec![] };
    let span = m.span();
    m.traces = vec![plot::trace(&m.s, span, false), plot::trace(&m.t, span, true)];
    Some(m)
}

impl App {
    fn tr(&self, key: &str) -> &'static str {
        tr(self.lang, key, self.inp.linear)
    }

    fn scene(&self) -> Scene {
        let (p, st) = self.m.active(self.inp.kind);
        let s = p.sample(self.t);
        let tint = |a: f64, peak: f64| if peak > 0.0 { (self.m.load.motor_torque(a).abs() / peak) as f32 } else { 0.0 };
        Scene {
            linear: self.inp.linear,
            d: self.inp.d,
            p: s.p,
            v: s.v,
            tint: tint(s.a, st.peak_torque),
            axis2: self.m.axis2.as_ref().map(|a2| {
                let s2 = a2.profile.sample(self.t);
                let pk = load_stats(&a2.profile, &self.m.load).peak_torque;
                (self.inp.d2, s2.p, s2.v, tint(s2.a, pk))
            }),
        }
    }

    /// Re-read the form and recompute. Keeps the last valid model on bad input.
    fn update(&mut self) {
        self.inp = read_inputs();
        match compute(&self.inp) {
            Some(m) => {
                self.m = m;
                self.err = false;
            }
            None => self.err = true,
        }
        self.t = self.t.min(self.m.t_end(self.inp.kind));
        self.refresh_text();
    }

    fn apply_lang(&mut self) {
        let d = doc();
        if let Some(h) = d.document_element() {
            let _ = h.set_attribute("lang", if self.lang == Lang::Ko { "ko" } else { "en" });
        }
        d.set_title(self.tr("title"));
        let list = d.query_selector_all("[data-i18n]").unwrap();
        for k in 0..list.length() {
            if let Some(e) = list.item(k).and_then(|n| n.dyn_into::<Element>().ok()) {
                let key = e.get_attribute("data-i18n").unwrap_or_default();
                e.set_text_content(Some(self.tr(&key)));
            }
        }
        let list = d.query_selector_all("[data-unit]").unwrap();
        for k in 0..list.length() {
            if let Some(e) = list.item(k).and_then(|n| n.dyn_into::<Element>().ok()) {
                let key = e.get_attribute("data-unit").unwrap_or_default();
                e.set_text_content(Some(unit(&key, self.inp.linear)));
            }
        }
        self.refresh_text();
    }

    fn refresh_text(&mut self) {
        el("play").set_text_content(Some(self.tr(if self.play { "pause" } else { "play" })));
        el("err").set_text_content(Some(if self.err { self.tr("err") } else { "" }));
        el("sync-row").class_list().toggle_with_force("off", !self.inp.sync).ok();
        el("tg-axis2").class_list().toggle_with_force("off", !self.inp.sync).ok();
        el("results").set_inner_html(&self.results_html());
    }

    fn results_html(&self) -> String {
        let (m, lin) = (&self.m, self.inp.linear);
        let (vel, acc) = (unit("vel", lin), unit("acc", lin));
        let rpm = |w: f64| w * 60.0 / std::f64::consts::TAU;
        let mut h = format!("<table><tr><th></th><th>{}</th><th>{}</th></tr>", self.tr("col_s"), self.tr("col_t"));
        let mut row = |k: &str, a: f64, b: f64, u: &str| {
            h += &format!("<tr><td>{}</td><td>{} {u}</td><td>{} {u}</td></tr>", self.tr(k), num(a), num(b));
        };
        row("r_total", m.s.total(), m.t.total(), "s");
        row("r_tj", m.s.tj, m.t.tj, "s");
        row("r_ta", m.s.ta, m.t.ta, "s");
        row("r_tv", m.s.tv, m.t.tv, "s");
        row("r_vpk", m.s.v_peak, m.t.v_peak, vel);
        row("r_apk", m.s.a_peak, m.t.a_peak, acc);
        row("r_tpk", m.ls.peak_torque, m.lt.peak_torque, "N·m");
        row("r_trms", m.ls.rms_torque, m.lt.rms_torque, "N·m");
        row("r_pow", m.ls.peak_power, m.lt.peak_power, "W");
        row("r_spd", rpm(m.ls.peak_speed), rpm(m.lt.peak_speed), "rpm");
        let check = |st: &LoadStats| {
            let ok = st.rms_torque <= self.inp.rated;
            let pct = if self.inp.rated > 0.0 { st.rms_torque / self.inp.rated * 100.0 } else { f64::INFINITY };
            format!("<span class=\"{}\">{} {}% {}</span>", if ok { "ok" } else { "bad" }, if ok { "✅" } else { "⚠️" }, num(pct), self.tr(if ok { "r_ok" } else { "r_over" }))
        };
        h += &format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>", self.tr("r_rated"), check(&m.ls), check(&m.lt));
        h += &format!("<tr><td>{}</td><td colspan=\"2\">+{} %</td></tr>", self.tr("r_pen"), num(time_penalty(&m.s, &m.t)));
        if let Some(a2) = &m.axis2 {
            let target = m.active(self.inp.kind).0.total();
            let msg = if a2.ok { self.tr("r_sync_ok") } else { self.tr("r_sync_slow") };
            let msg = msg.replace("{n}", &num(a2.natural)).replace("{t}", &num(target)).replace("{v}", &format!("{} {vel}", num(a2.profile.v_peak)));
            h += &format!("<tr><td>{}</td><td colspan=\"2\" class=\"{}\">{msg}</td></tr>", self.tr("r_sync"), if a2.ok { "ok" } else { "bad" });
        }
        h + "</table>"
    }

    fn frame(&mut self, now: f64) {
        let dt = ((now - self.last) / 1000.0).clamp(0.0, 0.1);
        self.last = now;
        let t_end = self.m.t_end(self.inp.kind);
        if self.play {
            self.t += dt * self.speed;
            if self.t > t_end {
                if self.looping {
                    self.t = 0.0;
                } else {
                    self.t = t_end;
                    self.play = false;
                    self.refresh_text();
                }
            }
        }
        let sc = self.scene();
        if let Some(v) = &mut self.view {
            v.frame(&sc, dt as f32);
        }
        let k = self.inp.kind;
        let (p, _) = self.m.active(k);
        let s = p.sample(self.t);
        let lin = self.inp.linear;
        let hud = format!(
            "t  {} s   seg {}\np  {} {}\nv  {} {}\na  {} {}\nτ_m {} N·m",
            num(self.t),
            p.segment_at(self.t),
            num(s.p),
            unit("pos", lin),
            num(s.v),
            unit("vel", lin),
            num(s.a),
            unit("acc", lin),
            num(self.m.load.motor_torque(s.a))
        );
        // Key includes t_end: early in the move the HUD text is independent of D,
        // but the slider label must still follow a new total time.
        let key = format!("{hud}|{t_end}");
        if key != self.hud {
            el("hud").set_text_content(Some(&hud));
            input("time").set_value(&format!("{}", if t_end > 0.0 { (self.t / t_end * 1000.0).round() } else { 0.0 }));
            el("tlabel").set_text_content(Some(&format!("{} / {} s", num(self.t), num(t_end))));
            self.hud = key;
        }
        let labels = [
            format!("p ({})", unit("pos", lin)),
            format!("v ({})", unit("vel", lin)),
            format!("a ({})", unit("acc", lin)),
            format!("j ({})", unit("jerk", lin)),
        ];
        let st = plot::Style {
            dark: self.dark,
            labels: [&labels[0], &labels[1], &labels[2], &labels[3]],
            legend: [self.tr("col_s"), self.tr("col_t")],
        };
        plot::draw(&el("plot").unchecked_into(), &self.m.traces, self.m.span(), self.t, &st);
    }

    fn load_preset(&mut self, p: &Inputs) {
        write_inputs(p);
        self.t = 0.0;
        self.update();
        self.apply_lang();
        let sc = self.scene();
        if let Some(v) = &mut self.view {
            v.reset(&sc);
        }
    }
}

fn pointer_xy(e: &PointerEvent) -> (f32, f32) {
    (e.offset_x() as f32, e.offset_y() as f32)
}

fn set_toggle(id: &str, on: bool) {
    let _ = el(id).set_attribute("aria-pressed", if on { "true" } else { "false" });
}

#[wasm_bindgen(start)]
pub fn start() {
    let win = web_sys::window().unwrap();
    let lang = if win.navigator().language().unwrap_or_default().to_lowercase().starts_with("ko") { Lang::Ko } else { Lang::En };
    let dark_q = win.match_media("(prefers-color-scheme: dark)").ok().flatten();
    let dark = dark_q.as_ref().is_some_and(|q| q.matches());
    let canvas: HtmlCanvasElement = el("gl").unchecked_into();
    let view = View::new(&canvas, dark);
    if view.is_none() {
        el("nogl").class_list().remove_1("off").ok();
        el("overlay").class_list().add_1("off").ok();
    }
    write_inputs(&KNEE);
    let inp = read_inputs();
    let m = compute(&inp).expect("preset is valid");
    let app = App { lang, inp, m, err: false, play: true, looping: true, speed: 0.25, t: 0.0, last: 0.0, view, dark, hud: String::new() };
    APP.with(|a| *a.borrow_mut() = Some(app));
    with(|a| {
        a.apply_lang();
        let sc = a.scene();
        if let Some(v) = &mut a.view {
            v.reset(&sc);
        }
    });

    // Form.
    for id in NUM_IDS.iter().chain(&["sync", "kind"]) {
        on_id(id, "input", |_| {
            with(|a| {
                select("preset").set_value("custom");
                a.update();
            });
        });
    }
    on_id("mode", "change", |_| {
        with(|a| {
            select("preset").set_value("custom");
            a.update();
            a.apply_lang();
            let sc = a.scene();
            if let Some(v) = &mut a.view {
                v.reset(&sc);
            }
        });
    });
    on_id("preset", "change", |_| {
        let p = match select("preset").value().as_str() {
            "knee" => KNEE,
            "gantry" => GANTRY,
            _ => return,
        };
        with(|a| a.load_preset(&p));
    });
    on_id("lang", "click", |_| {
        with(|a| {
            a.lang = if a.lang == Lang::Ko { Lang::En } else { Lang::Ko };
            a.apply_lang();
        });
    });

    // Playback.
    on_id("play", "click", |_| {
        with(|a| {
            if !a.play && a.t >= a.m.t_end(a.inp.kind) {
                a.t = 0.0;
            }
            a.play = !a.play;
            a.refresh_text();
        });
    });
    on_id("speed", "change", |_| {
        with(|a| a.speed = select("speed").value().parse().unwrap_or(0.25));
    });
    on_id("loop", "change", |_| {
        with(|a| a.looping = input("loop").checked());
    });
    on_id("time", "input", |_| {
        with(|a| {
            a.play = false;
            a.t = input("time").value_as_number() / 1000.0 * a.m.t_end(a.inp.kind);
            a.refresh_text();
        });
    });

    // View toggles and presets.
    for (id, which) in [("tg-ghost", 0), ("tg-vel", 1), ("tg-tint", 2), ("tg-axis2", 3)] {
        on_id(id, "click", move |_| {
            with(|a| {
                if let Some(v) = &mut a.view {
                    let t = &mut v.toggles;
                    let f = [&mut t.ghosts, &mut t.vel, &mut t.tint, &mut t.axis2];
                    let b = f.into_iter().nth(which).unwrap();
                    *b = !*b;
                    set_toggle(id, *b);
                }
            });
        });
    }
    for (id, p) in [("vp-persp", Preset::Persp), ("vp-top", Preset::Top), ("vp-side", Preset::Side), ("vp-front", Preset::Front)] {
        on_id(id, "click", move |_| {
            with(|a| {
                let sc = a.scene();
                if let Some(v) = &mut a.view {
                    v.set_preset(p, &sc);
                }
            });
        });
    }

    // Canvas interaction.
    let cv: &web_sys::EventTarget = canvas.as_ref();
    on(cv, "pointerdown", |e| {
        let e: PointerEvent = e.unchecked_into();
        e.prevent_default();
        let (x, y) = pointer_xy(&e);
        with(|a| {
            let sc = a.scene();
            if let Some(v) = &mut a.view {
                v.down(e.pointer_id(), x, y, e.button(), &sc);
            }
        });
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
    });
    on(cv, "pointermove", |e| {
        let e: PointerEvent = e.unchecked_into();
        let (x, y) = pointer_xy(&e);
        with(|a| {
            let sc = a.scene();
            let Some(v) = &mut a.view else { return };
            let nd = v.moved(e.pointer_id(), x, y, &sc);
            let cursor = if v.dragging() { "grabbing" } else if v.hover { "grab" } else { "default" };
            let _ = el("gl").unchecked_ref::<HtmlElement>().style().set_property("cursor", cursor);
            if let Some(d) = nd {
                if d != a.inp.d {
                    input("d").set_value(&format!("{d}"));
                    select("preset").set_value("custom");
                    a.update();
                }
            }
        });
    });
    for ev in ["pointerup", "pointercancel"] {
        on(cv, ev, |e| {
            let e: PointerEvent = e.unchecked_into();
            with(|a| {
                if let Some(v) = &mut a.view {
                    v.up(e.pointer_id());
                }
            });
        });
    }
    on(cv, "wheel", |e| {
        let e: WheelEvent = e.unchecked_into();
        e.prevent_default();
        with(|a| {
            if let Some(v) = &mut a.view {
                v.wheel(e.delta_y() as f32);
            }
        });
    });
    on(cv, "dblclick", |_| {
        with(|a| {
            let sc = a.scene();
            if let Some(v) = &mut a.view {
                v.reset(&sc);
            }
        });
    });
    on(cv, "contextmenu", |e| e.prevent_default());
    if let Some(q) = dark_q {
        let q2 = q.clone();
        on(&q, "change", move |_| {
            with(|a| {
                a.dark = q2.matches();
                if let Some(v) = &mut a.view {
                    v.set_dark(a.dark);
                }
            });
        });
    }

    // Animation loop.
    let cb: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    *cb.borrow_mut() = Some(Closure::new(move |now: f64| {
        with(|a| a.frame(now));
        if let Some(c) = cb2.borrow().as_ref() {
            let _ = web_sys::window().unwrap().request_animation_frame(c.as_ref().unchecked_ref());
        }
    }));
    if let Some(c) = cb.borrow().as_ref() {
        let _ = win.request_animation_frame(c.as_ref().unchecked_ref());
    };
}
