# Task: S-curve motion profile lab for robot joints & linear axes (Rust → WebAssembly, static web app)

Original open-source tool. Write all code yourself; do not copy anyone's code, tables or assets.
Use textbook trapezoidal and jerk-limited (7-segment, "S-curve") motion profile equations only.
Presets are generic, clearly labelled "typical" values that users can edit (no vendor names).

## Product (SMALL scope — core + UI about 800-1200 lines of new Rust; renderer/math are reused)
A single-page web app (GitHub Pages, sub-path `/s-curve/`): "My joint / axis must move this distance
with these velocity, acceleration and jerk limits — how long does the move take, what do position,
velocity, acceleration and jerk look like, and what peak torque / force does the motor need?"

1. **Inputs**: move distance D (rad for rotary or m for linear; mode toggle), start/end at rest,
   v_max, a_max, j_max, profile type (trapezoidal vs. jerk-limited S-curve), and for the load estimate:
   inertia J (kg·m²) or mass m (kg), constant load torque/force (gravity/friction), gear ratio N and
   gear efficiency η. Two presets: "humanoid knee joint (typical)" and "3D-printer / gantry linear axis
   (typical)" (editable).
2. **Profile model (core)**: exact time-optimal 7-segment jerk-limited profile for rest-to-rest moves,
   including the degenerate cases: v_max not reached (no cruise), a_max not reached (triangular
   acceleration), both. Trapezoidal (infinite jerk) as the comparison profile. Output: segment
   durations T_j, T_a, T_v, total time, peak v / a actually reached, sampled p(t), v(t), a(t), j(t)
   (closed-form evaluation per segment, not numerical integration).
   Load: motor-side torque τ_m(t) = (J·a(t) + τ_load) / (N·η), motor speed ω_m = N·v; report peak and
   RMS torque, peak mechanical power, and the time penalty of S-curve vs trapezoid (%).
3. **Extras (small)**: (a) a "sync" option — second axis with its own distance whose profile is
   time-scaled to finish at the same time as axis 1; (b) RMS-torque check against a user "rated torque"
   field (✅/⚠️).
4. **3D view (WebGL2, game-grade — see below)**: a simple robot link on a revolute joint (rotary mode:
   motor housing cylinder + gearbox ring + link capsule with an end-effector sphere, rotating by p(t))
   or a carriage on a rail (linear mode: rail box, carriage box, belt line), shown on the checker
   floor; translucent ghost of the start pose and target pose; an arc/arrow showing velocity;
   the motor housing tinted from neutral to red by |τ_m| / peak.
5. **2D plots** (canvas 2D, own small module): stacked p, v, a, j vs time for both profile types
   (S-curve solid, trapezoid dashed), with the current time as a vertical cursor.
6. Korean + English toggle (browser default). Mobile friendly. CSS prefers-color-scheme only.
Out of scope (do NOT build): non-zero start/end velocity, multi-waypoint trajectories, inverse
kinematics, motor databases, URL share, JSON import/export.

## 3D view requirements (game-like, reuse the existing renderer)
- `crates/sc_web/src/gl.rs` and `crates/sc_web/src/math.rs` already exist: a tool-independent WebGL2
  renderer (lit meshes, checker/grid ground with fog, sky gradient, stencil planar shadows, translucent
  volumes, lines) and pure math (mat4, damped orbit camera with inertia, ray-plane/ray-sphere picking)
  with unit tests. Reuse them; extend only if needed and keep them tool-independent and tested.
- Scene code in `view3d.rs`. Orbit (left drag) / pan (right drag, two fingers) / zoom (wheel, pinch)
  with inertia, double-click reset, `touch-action:none`. View presets: Perspective / Top / Side / Front.
- Direct manipulation: drag the **target ghost** (rotary: around the joint axis on its plane; linear:
  along the rail) to set the move distance D; the profile recomputes live. Hover highlight + cursor
  change on the draggable ghost.
- Toggle buttons over the view: ghosts, velocity arrow, torque tint, second axis (when sync on).
- Playback: play/pause, speed (0.1× / 0.25× / 1×, default 0.25×), time slider, loop; same instant shown
  as a cursor on the 2D plots. HUD over the canvas: t, segment (1-7), p, v, a, τ_m.
- If WebGL2 is unavailable, show a message and keep the 2D plots working.
- Release wasm ≲ 350 KB. No crates besides wasm-bindgen, web-sys, js-sys.

## Core crate (pure Rust, no web deps, unit tested) — `crates/sc_core`
- Profile planner (S-curve + trapezoid), closed-form sampling, load/torque calculation, time sync.
- Tests (hand-checkable, 6-10): trapezoid D=1, v=1, a=1 → T=2 s (triangle exactly at v_max);
  trapezoid D=4, v=1, a=1 → T=5 s; S-curve D large with v=1, a=1, j=1 → T_j=1, T_a=2, total =
  D/v + T_a + ... (check against hand formula T = D/v + v/a + a/j); short move where a_max is not
  reached → peak a < a_max and p(T) = D within 1e-9; p(t) continuous at all segment boundaries and
  v(T)=a(T)=0; S-curve total time ≥ trapezoid time for the same limits; torque τ = (J·a+τ_load)/(N·η)
  hand example; sync stretches axis 2 to the same total time with peak v ≤ its v_max.
- `crates/sc_web`: UI, plain web-sys.

## Repo hygiene
- README (Korean first, then English): what it is, model and assumptions (rest-to-rest, rigid load,
  constant inertia, no motor dynamics/current limits), controls, local build (`cargo test`,
  `trunk build --release`), MIT license, "educational estimates — validate before using on a real robot".
- LICENSE: MIT, copyright "s-curve contributors". `.gitignore` for target/ dist/.
- `index.html` with a CSP meta that blocks external requests:
  `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'self'`.
  In `Trunk.toml` set `filehash = false`, `public_url = "/s-curve/"` and
  `pattern_script = "<script type=\"module\" src=\"{base}boot.js\"></script>"` with an external
  `web/boot.js` (copied via `<link data-trunk rel="copy-file">`) that imports `./sc_web.js` and calls init.
- `.github/workflows/pages.yml`: on push to main + workflow_dispatch; checkout, rust stable + wasm32
  target, install trunk 0.22.1 binary release, `sh tools/privacy_check.sh`, `cargo test --release`,
  `trunk build --release`, configure-pages@v5, upload-pages-artifact@v3 (dist), deploy job with
  actions/deploy-pages@v4 (permissions pages: write, id-token: write).
- `tools/privacy_check.sh` already exists; keep it and keep it clean.

## Done when
- `cargo test` green (workspace); `trunk build --release` succeeds with no warnings;
  `sh tools/privacy_check.sh` clean.
- Commit author `s-curve contributors <59640857+roscore@users.noreply.github.com>`.
- Commit everything with clear commits; open a PR. Do NOT create new GitHub repos or change repo settings.
