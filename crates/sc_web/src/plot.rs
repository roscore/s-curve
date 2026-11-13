//! Stacked p / v / a / j vs time plots on a 2D canvas: S-curve solid, trapezoid dashed,
//! the current time as a vertical cursor.
use sc_core::{Profile, State};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d as Ctx, HtmlCanvasElement};

const N: usize = 400;

/// Pre-sampled curve of one profile.
pub struct Trace {
    pub pts: Vec<(f64, State)>,
    pub dashed: bool,
    /// The trapezoid's jerk is impulsive and not drawn.
    pub has_jerk: bool,
}

pub fn trace(p: &Profile, t_end: f64, dashed: bool) -> Trace {
    let t_end = t_end.max(1e-9);
    // Uniform samples plus both sides of each segment boundary, so steps stay sharp.
    let mut ts: Vec<f64> = (0..=N).map(|i| t_end * i as f64 / N as f64).collect();
    let mut t0 = 0.0;
    for g in &p.segs {
        t0 += g.dur;
        ts.extend([t0 - 1e-9, t0 + 1e-9]);
    }
    ts.retain(|t| (0.0..=t_end).contains(t));
    ts.sort_by(|a, b| a.total_cmp(b));
    Trace { pts: ts.into_iter().map(|t| (t, p.sample(t))).collect(), dashed, has_jerk: p.kind == sc_core::Kind::SCurve }
}

pub struct Style<'a> {
    pub dark: bool,
    /// Row labels with units, e.g. "p (rad)".
    pub labels: [&'a str; 4],
    pub legend: [&'a str; 2],
}

fn fmt(x: f64) -> String {
    let a = x.abs();
    if a >= 100.0 {
        format!("{x:.0}")
    } else if a >= 10.0 {
        format!("{x:.1}")
    } else if a >= 1.0 {
        format!("{x:.2}")
    } else {
        format!("{x:.3}")
    }
}

/// Redraw all four panels. `traces[0]` is the S-curve, `traces[1]` the trapezoid.
pub fn draw(cv: &HtmlCanvasElement, traces: &[Trace], t_end: f64, cursor: f64, st: &Style) -> Option<()> {
    let dpr = web_sys::window()?.device_pixel_ratio().min(2.0);
    let (cw, ch) = (cv.client_width() as f64, cv.client_height() as f64);
    if cw < 10.0 || ch < 10.0 {
        return Some(());
    }
    let (w, h) = ((cw * dpr) as u32, (ch * dpr) as u32);
    if cv.width() != w || cv.height() != h {
        cv.set_width(w);
        cv.set_height(h);
    }
    let c: Ctx = cv.get_context("2d").ok()??.dyn_into().ok()?;
    c.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).ok()?;
    c.clear_rect(0.0, 0.0, cw, ch);
    let (fg, grid) = if st.dark { ("#c9ced8", "#3a3f4a") } else { ("#30343c", "#d5d8de") };
    let colors = ["#3b8eea", "#2fae66", "#e8862a", "#a86ad8"];
    let (l, r, top, bottom) = (52.0, 10.0, 22.0, 26.0);
    let ph = (ch - top - bottom) / 4.0;
    let pw = cw - l - r;
    let t_end = t_end.max(1e-9);
    let xs = |t: f64| l + t / t_end * pw;
    c.set_font("11px system-ui, sans-serif");
    c.set_line_width(1.0);

    // Legend.
    c.set_fill_style_str(fg);
    c.set_stroke_style_str(fg);
    for (i, name) in st.legend.iter().enumerate() {
        let x = l + i as f64 * 120.0;
        c.set_line_dash(&dash(i == 1)).ok()?;
        c.begin_path();
        c.move_to(x, 11.0);
        c.line_to(x + 22.0, 11.0);
        c.stroke();
        c.fill_text(name, x + 27.0, 15.0).ok()?;
    }
    c.set_line_dash(&dash(false)).ok()?;

    for row in 0..4 {
        let y0 = top + row as f64 * ph;
        let get = |s: &State| [s.p, s.v, s.a, s.j][row];
        let used = |tr: &&Trace| row < 3 || tr.has_jerk;
        let (mut lo, mut hi) = (0.0f64, 0.0f64);
        for tr in traces.iter().filter(used) {
            for (_, s) in &tr.pts {
                lo = lo.min(get(s));
                hi = hi.max(get(s));
            }
        }
        if hi - lo < 1e-12 {
            hi += 1.0;
        }
        let pad = (hi - lo) * 0.08;
        let (lo, hi) = (lo - pad, hi + pad);
        let ys = |v: f64| y0 + 4.0 + (hi - v) / (hi - lo) * (ph - 10.0);
        // Frame, zero line, labels.
        c.set_stroke_style_str(grid);
        c.stroke_rect(l, y0 + 2.0, pw, ph - 6.0);
        c.begin_path();
        c.move_to(l, ys(0.0));
        c.line_to(l + pw, ys(0.0));
        c.stroke();
        c.set_fill_style_str(fg);
        c.fill_text(st.labels[row], l + 4.0, y0 + 14.0).ok()?;
        c.fill_text(&fmt(hi - pad), 4.0, y0 + 14.0).ok()?;
        c.fill_text(&fmt(lo + pad), 4.0, y0 + ph - 6.0).ok()?;
        // Curves.
        c.set_stroke_style_str(colors[row]);
        c.set_line_width(1.6);
        for tr in traces.iter().filter(used) {
            c.set_line_dash(&dash(tr.dashed)).ok()?;
            c.begin_path();
            for (i, (t, s)) in tr.pts.iter().enumerate() {
                if i == 0 {
                    c.move_to(xs(*t), ys(get(s)));
                } else {
                    c.line_to(xs(*t), ys(get(s)));
                }
            }
            c.stroke();
        }
        c.set_line_dash(&dash(false)).ok()?;
        c.set_line_width(1.0);
    }
    // Time axis ticks.
    c.set_fill_style_str(fg);
    let step = nice_step(t_end / 6.0);
    let mut t = 0.0;
    while t <= t_end + 1e-9 {
        c.fill_text(&format!("{}", fmt(t)), xs(t) - 8.0, ch - 8.0).ok()?;
        t += step;
    }
    c.fill_text("t (s)", l + pw - 26.0, ch - 8.0).ok()?;
    // Cursor.
    c.set_stroke_style_str("#e5484d");
    c.begin_path();
    c.move_to(xs(cursor.clamp(0.0, t_end)), top);
    c.line_to(xs(cursor.clamp(0.0, t_end)), top + 4.0 * ph - 4.0);
    c.stroke();
    Some(())
}

fn dash(on: bool) -> JsValue {
    let a = js_sys::Array::new();
    if on {
        a.push(&6.0.into());
        a.push(&4.0.into());
    }
    a.into()
}

fn nice_step(x: f64) -> f64 {
    let e = 10f64.powf(x.max(1e-9).log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * e).find(|s| *s >= x).unwrap_or(10.0 * e)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nice_steps() {
        assert_eq!(super::nice_step(0.3), 0.5);
        assert_eq!(super::nice_step(1.0), 1.0);
        assert_eq!(super::nice_step(1.3), 2.0);
        assert_eq!(super::nice_step(7.0), 10.0);
    }
}
