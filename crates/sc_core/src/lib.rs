//! Rest-to-rest motion profiles for a single axis (rotary joint or linear axis):
//!
//! * jerk-limited, time-optimal 7-segment "S-curve" (symmetric acceleration / deceleration),
//! * trapezoidal velocity (infinite jerk) for comparison,
//!
//! with closed-form per-segment sampling, load torque / force estimates and time synchronisation
//! of a second axis. Units are generic: rad (rotary) or m (linear) for the position.
//! Pure Rust, no dependencies.

/// Kinematic limits (all strictly positive). `j` is ignored by the trapezoidal profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Jerk-limited 7-segment profile.
    SCurve,
    /// Trapezoidal velocity profile (acceleration steps, jerk impulses).
    Trapezoid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// A limit is zero, negative or not finite.
    BadLimit,
    /// The distance is not finite.
    BadDistance,
}

/// One constant-jerk piece, with the state at its start.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    pub dur: f64,
    pub jerk: f64,
    pub p0: f64,
    pub v0: f64,
    pub a0: f64,
}

/// Instantaneous kinematic state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub p: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

/// A planned rest-to-rest move. Segments 1-7 are: jerk up, constant acceleration, jerk down,
/// cruise, jerk down, constant deceleration, jerk up. The trapezoid has zero-length jerk segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub kind: Kind,
    /// Signed distance D.
    pub dist: f64,
    /// Duration of each jerk phase (0 for the trapezoid).
    pub tj: f64,
    /// Duration of the whole acceleration phase (segments 1-3); deceleration is the same.
    pub ta: f64,
    /// Duration of the cruise phase (segment 4).
    pub tv: f64,
    /// Peak |velocity| and |acceleration| actually reached.
    pub v_peak: f64,
    pub a_peak: f64,
    pub segs: [Segment; 7],
}

fn valid(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

impl Profile {
    /// Time-optimal rest-to-rest profile for distance `d` within `lim`.
    pub fn plan(kind: Kind, d: f64, lim: Limits) -> Result<Profile, PlanError> {
        if !d.is_finite() {
            return Err(PlanError::BadDistance);
        }
        let jerk_ok = kind == Kind::Trapezoid || valid(lim.j);
        if !(valid(lim.v) && valid(lim.a) && jerk_ok) {
            return Err(PlanError::BadLimit);
        }
        let (s, h) = (d.signum(), d.abs());
        let Limits { v, a, j } = lim;
        let (tj, ta, tv, ap) = match kind {
            Kind::Trapezoid => {
                let (ta, tv) = if h >= v * v / a { (v / a, h / v - v / a) } else { ((h / a).sqrt(), 0.0) };
                (0.0, ta, tv, a)
            }
            Kind::SCurve => {
                // Assume v_max is reached: is a_max reached on the way?
                let (tj, ta) = if v * j >= a * a { (a / j, a / j + v / a) } else { ((v / j).sqrt(), 2.0 * (v / j).sqrt()) };
                let tv = h / v - ta;
                if tv >= 0.0 {
                    (tj, ta, tv, j * tj)
                } else {
                    // No cruise. With a_max reached: D = a (Ta − Tj) Ta.
                    let tj = a / j;
                    let ta = 0.5 * (tj + (tj * tj + 4.0 * h / a).sqrt());
                    if ta >= 2.0 * tj {
                        (tj, ta, 0.0, a)
                    } else {
                        // Neither limit reached: D = 2 j Tj³.
                        let tj = (h / (2.0 * j)).cbrt();
                        (tj, 2.0 * tj, 0.0, j * tj)
                    }
                }
            }
        };
        let jk = if kind == Kind::SCurve { j } else { 0.0 };
        let tc = (ta - 2.0 * tj).max(0.0);
        let durs = [tj, tc, tj, tv, tj, tc, tj];
        let jerks = [jk, 0.0, -jk, 0.0, -jk, 0.0, jk].map(|x| s * x);
        let accs = match kind {
            Kind::SCurve => [0.0, ap, ap, 0.0, 0.0, -ap, -ap],
            Kind::Trapezoid => [0.0, ap, 0.0, 0.0, 0.0, -ap, 0.0],
        }
        .map(|x| s * x);
        let mut segs = [Segment::default(); 7];
        let (mut p, mut vel) = (0.0, 0.0);
        for i in 0..7 {
            let g = Segment { dur: durs[i], jerk: jerks[i], p0: p, v0: vel, a0: accs[i] };
            let e = g.eval(g.dur);
            (p, vel) = (e.p, e.v);
            segs[i] = g;
        }
        Ok(Profile { kind, dist: d, tj, ta, tv, v_peak: ap * (ta - tj), a_peak: if h > 0.0 { ap } else { 0.0 }, segs })
    }

    /// Total move time.
    pub fn total(&self) -> f64 {
        self.segs.iter().map(|g| g.dur).sum()
    }

    /// Segment number (1-7) active at time `t`; boundaries belong to the next non-empty segment.
    pub fn segment_at(&self, t: f64) -> usize {
        let mut end = 0.0;
        for (i, g) in self.segs.iter().enumerate() {
            end += g.dur;
            if g.dur > 0.0 && t < end {
                return i + 1;
            }
        }
        7
    }

    /// Closed-form state at time `t`: rest at 0 before the move, rest at D after it.
    pub fn sample(&self, t: f64) -> State {
        if t <= 0.0 {
            return State::default();
        }
        let mut t0 = 0.0;
        for g in &self.segs {
            if g.dur > 0.0 && t < t0 + g.dur {
                return g.eval(t - t0);
            }
            t0 += g.dur;
        }
        State { p: self.dist, ..State::default() }
    }

    /// The same path slowed down to last `t_new` (≥ the current time): durations × k,
    /// velocity / k, acceleration / k², jerk / k³ with k = t_new / T. Never compresses.
    pub fn stretched(&self, t_new: f64) -> Profile {
        let t = self.total();
        if t <= 0.0 || !(t_new > t) {
            return self.clone();
        }
        let k = t_new / t;
        let mut out = self.clone();
        for g in &mut out.segs {
            *g = Segment { dur: g.dur * k, jerk: g.jerk / (k * k * k), p0: g.p0, v0: g.v0 / k, a0: g.a0 / (k * k) };
        }
        (out.tj, out.ta, out.tv) = (self.tj * k, self.ta * k, self.tv * k);
        (out.v_peak, out.a_peak) = (self.v_peak / k, self.a_peak / (k * k));
        out
    }
}

impl Segment {
    /// State `dt` seconds into this segment (cubic in time).
    pub fn eval(&self, dt: f64) -> State {
        let (j, a, v) = (self.jerk, self.a0, self.v0);
        State {
            p: self.p0 + dt * (v + dt * (a / 2.0 + dt * j / 6.0)),
            v: v + dt * (a + dt * j / 2.0),
            a: a + dt * j,
            j,
        }
    }
}

/// Second axis that has to finish together with axis 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Synced {
    pub profile: Profile,
    /// Its own time-optimal duration.
    pub natural: f64,
    /// False when axis 2 cannot finish within the target time (it is the bottleneck).
    pub ok: bool,
}

/// Plan axis 2 over `d2` and stretch it to `t_target` (the duration of axis 1).
pub fn sync(kind: Kind, d2: f64, lim: Limits, t_target: f64) -> Result<Synced, PlanError> {
    let p = Profile::plan(kind, d2, lim)?;
    let natural = p.total();
    let ok = natural <= t_target * (1.0 + 1e-12);
    Ok(Synced { profile: if ok { p.stretched(t_target) } else { p }, natural, ok })
}

/// Rigid load behind a gear (rotary: torque / inertia, linear: force / mass).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    /// Load inertia J (kg·m²) or mass m (kg).
    pub inertia: f64,
    /// Constant load torque (N·m) or force (N): gravity, friction.
    pub load: f64,
    /// Gear / transmission ratio N: motor rad per load rad (rotary) or per metre (linear, rad/m).
    pub ratio: f64,
    /// Transmission efficiency η (0-1].
    pub eff: f64,
}

impl Load {
    /// Motor-side torque τ_m = (J·a + τ_load) / (N·η).
    pub fn motor_torque(&self, a: f64) -> f64 {
        (self.inertia * a + self.load) / (self.ratio * self.eff)
    }
    /// Motor speed ω_m = N·v (rad/s).
    pub fn motor_speed(&self, v: f64) -> f64 {
        self.ratio * v
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LoadStats {
    pub peak_torque: f64,
    pub rms_torque: f64,
    /// Peak |τ_m · ω_m| (W).
    pub peak_power: f64,
    /// Peak |ω_m| (rad/s).
    pub peak_speed: f64,
}

/// Peak and RMS motor torque over the move, peak mechanical power and motor speed.
/// The RMS is exact: τ_m is linear in a, and a is linear within each segment.
pub fn load_stats(p: &Profile, l: &Load) -> LoadStats {
    let t = p.total();
    let mut st = LoadStats { peak_torque: l.motor_torque(0.0).abs(), peak_speed: l.motor_speed(p.v_peak).abs(), ..LoadStats::default() };
    let mut sq = 0.0;
    for g in p.segs.iter().filter(|g| g.dur > 0.0) {
        let (u0, u1) = (l.motor_torque(g.a0), l.motor_torque(g.eval(g.dur).a));
        st.peak_torque = st.peak_torque.max(u0.abs()).max(u1.abs());
        sq += g.dur * (u0 * u0 + u0 * u1 + u1 * u1) / 3.0;
        for k in 0..=64 {
            let s = g.eval(g.dur * k as f64 / 64.0);
            st.peak_power = st.peak_power.max((l.motor_torque(s.a) * l.motor_speed(s.v)).abs());
        }
    }
    st.rms_torque = if t > 0.0 { (sq / t).sqrt() } else { l.motor_torque(0.0).abs() };
    st
}

/// How much longer the S-curve takes than the trapezoid, in percent.
pub fn time_penalty(s_curve: &Profile, trapezoid: &Profile) -> f64 {
    let t = trapezoid.total();
    if t > 0.0 { (s_curve.total() / t - 1.0) * 100.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const L111: Limits = Limits { v: 1.0, a: 1.0, j: 1.0 };

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} vs {b}");
    }
    fn plan(k: Kind, d: f64, v: f64, a: f64, j: f64) -> Profile {
        Profile::plan(k, d, Limits { v, a, j }).unwrap()
    }
    /// End state, continuity across boundaries and limits.
    fn check(p: &Profile, lim: Limits) {
        let t = p.total();
        let e = p.segs[6].eval(p.segs[6].dur);
        close(e.p, p.dist, 1e-9 * p.dist.abs().max(1.0));
        close(e.v, 0.0, 1e-9 * lim.v.max(1.0));
        close(e.a, 0.0, 1e-9 * lim.a.max(1.0));
        let mut t0 = 0.0;
        for w in p.segs.windows(2) {
            let s = w[0].eval(w[0].dur);
            close(s.p, w[1].p0, 1e-9 * p.dist.abs().max(1.0));
            close(s.v, w[1].v0, 1e-9 * lim.v.max(1.0));
            if p.kind == Kind::SCurve {
                close(s.a, w[1].a0, 1e-9 * lim.a.max(1.0));
            }
            t0 += w[0].dur;
            let (b, a) = (p.sample(t0 - 1e-9), p.sample(t0 + 1e-9));
            close(b.p, a.p, 1e-6 * p.dist.abs().max(1.0));
        }
        let peak = (0..=2000).map(|i| p.sample(t * i as f64 / 2000.0)).fold((0.0f64, 0.0f64), |m, s| (m.0.max(s.v.abs()), m.1.max(s.a.abs())));
        assert!(peak.0 <= lim.v * (1.0 + 1e-9) && peak.1 <= lim.a * (1.0 + 1e-9), "{peak:?} {lim:?}");
        assert!(p.v_peak <= lim.v * (1.0 + 1e-9) && p.a_peak <= lim.a * (1.0 + 1e-9));
        assert_eq!(p.sample(t + 1.0), State { p: p.dist, ..State::default() });
    }

    #[test]
    fn trapezoid_hand_cases() {
        let p = plan(Kind::Trapezoid, 1.0, 1.0, 1.0, 1.0);
        close(p.total(), 2.0, 1e-12);
        close(p.v_peak, 1.0, 1e-12);
        close(p.tv, 0.0, 1e-12);
        let p = plan(Kind::Trapezoid, 4.0, 1.0, 1.0, 1.0);
        close(p.total(), 5.0, 1e-12);
        close(p.ta, 1.0, 1e-12);
        close(p.tv, 3.0, 1e-12);
        close(p.sample(2.5).p, 2.0, 1e-12);
        check(&p, L111);
    }

    #[test]
    fn s_curve_long_move_matches_hand_formula() {
        let d = 10.0;
        let p = plan(Kind::SCurve, d, 1.0, 1.0, 1.0);
        close(p.tj, 1.0, 1e-12);
        close(p.ta, 2.0, 1e-12);
        close(p.tv, d - 2.0, 1e-12);
        close(p.total(), d / 1.0 + 1.0 / 1.0 + 1.0 / 1.0, 1e-12); // D/v + v/a + a/j
        close(p.sample(5.0).v, 1.0, 1e-12);
        check(&p, L111);
    }

    #[test]
    fn s_curve_degenerate_regimes() {
        // a reached, v not reached (no cruise): Ta² − Ta − 4 = 0.
        let p = plan(Kind::SCurve, 4.0, 2.0, 1.0, 1.0);
        close(p.ta, (1.0 + 17f64.sqrt()) / 2.0, 1e-12);
        close(p.v_peak, p.ta - 1.0, 1e-12);
        close(p.tv, 0.0, 1e-12);
        check(&p, Limits { v: 2.0, a: 1.0, j: 1.0 });
        // v reached, a not reached (v·j < a²): Tj = √(v/j).
        let p = plan(Kind::SCurve, 5.0, 1.0, 2.0, 1.0);
        close(p.tj, 1.0, 1e-12);
        close(p.a_peak, 1.0, 1e-12);
        close(p.total(), 7.0, 1e-12);
        check(&p, Limits { v: 1.0, a: 2.0, j: 1.0 });
        // Neither reached: D = 2 j Tj³.
        let p = plan(Kind::SCurve, 0.25, 1.0, 1.0, 1.0);
        close(p.tj, 0.5, 1e-12);
        close(p.a_peak, 0.5, 1e-12);
        close(p.total(), 2.0, 1e-12);
        assert!(p.a_peak < 1.0);
        close(p.sample(p.total()).p, 0.25, 1e-9);
        check(&p, L111);
    }

    #[test]
    fn grid_sweep_end_state_continuity_and_ordering() {
        for &d in &[1e-4, 0.01, 0.3, 1.0, 2.5, 7.0, 40.0, -3.0] {
            for &v in &[0.2, 1.0, 5.0] {
                for &a in &[0.5, 2.0, 30.0] {
                    for &j in &[1.0, 10.0, 500.0] {
                        let lim = Limits { v, a, j };
                        let s = Profile::plan(Kind::SCurve, d, lim).unwrap();
                        let t = Profile::plan(Kind::Trapezoid, d, lim).unwrap();
                        check(&s, lim);
                        check(&t, lim);
                        assert!(s.total() >= t.total() - 1e-12, "{d} {lim:?}");
                        assert!(time_penalty(&s, &t) >= -1e-9);
                    }
                }
            }
        }
    }

    #[test]
    fn segments_and_errors() {
        let p = plan(Kind::SCurve, 10.0, 1.0, 1.0, 1.0);
        let seg: Vec<usize> = [0.0, 0.5, 1.5, 2.5, 9.5, 10.5, 11.0, 11.5, 13.0].iter().map(|&t| p.segment_at(t)).collect();
        assert_eq!(seg, [1, 1, 3, 4, 4, 5, 7, 7, 7]);
        assert_eq!(p.segs[1].dur, 0.0, "a_max reached exactly at the end of the jerk phase");
        let z = plan(Kind::SCurve, 0.0, 1.0, 1.0, 1.0);
        assert_eq!((z.total(), z.sample(1.0)), (0.0, State::default()));
        assert_eq!(Profile::plan(Kind::SCurve, 1.0, Limits { j: 0.0, ..L111 }), Err(PlanError::BadLimit));
        assert!(Profile::plan(Kind::Trapezoid, 1.0, Limits { j: 0.0, ..L111 }).is_ok());
        assert_eq!(Profile::plan(Kind::SCurve, f64::NAN, L111), Err(PlanError::BadDistance));
    }

    #[test]
    fn torque_hand_example() {
        // J = 0.5 kg·m², a = 4 rad/s², τ_load = 3 N·m, N = 10, η = 0.5 → τ_m = 5 / 5 = 1 N·m.
        let l = Load { inertia: 0.5, load: 3.0, ratio: 10.0, eff: 0.5 };
        close(l.motor_torque(4.0), 1.0, 1e-12);
        close(l.motor_speed(2.0), 20.0, 1e-12);
        // Trapezoid D=4, v=a=1: τ = (0.5·±1 + 3)/5 for 1 s each, 3/5 for 3 s.
        let p = plan(Kind::Trapezoid, 4.0, 1.0, 1.0, 1.0);
        let s = load_stats(&p, &l);
        close(s.peak_torque, 0.7, 1e-12);
        close(s.rms_torque, ((0.49 + 0.25 + 3.0 * 0.36) / 5.0f64).sqrt(), 1e-12);
        close(s.peak_speed, 10.0, 1e-12);
        close(s.peak_power, 0.7 * 10.0, 1e-9); // τ·ω at the end of the acceleration
    }

    #[test]
    fn sync_stretches_axis_two() {
        let lim = Limits { v: 1.0, a: 2.0, j: 10.0 };
        let a1 = plan(Kind::SCurve, 5.0, lim.v, lim.a, lim.j);
        let s = sync(Kind::SCurve, 1.5, lim, a1.total()).unwrap();
        assert!(s.ok && s.natural < a1.total());
        close(s.profile.total(), a1.total(), 1e-12);
        assert!(s.profile.v_peak <= lim.v);
        check(&s.profile, lim);
        let max_v = (0..=1000).map(|i| s.profile.sample(a1.total() * i as f64 / 1000.0).v).fold(0.0, f64::max);
        assert!(max_v <= s.profile.v_peak * (1.0 + 1e-9));
        // Axis 2 slower than axis 1: not compressed, flagged.
        let s = sync(Kind::SCurve, 20.0, lim, a1.total()).unwrap();
        assert!(!s.ok);
        close(s.profile.total(), s.natural, 1e-12);
    }
}
