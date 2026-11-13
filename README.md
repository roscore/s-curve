# s-curve

**한국어** · [English](#english)

로봇 관절과 직선 축을 위한 S-커브 모션 프로파일 실험실입니다. 브라우저에서만 동작하는 정적 웹 앱(Rust → WebAssembly)이에요.

> "이 관절(축)을 이 속도·가속도·저크 한계로 이만큼 움직여야 한다. 몇 초 걸리고, 위치·속도·가속도·저크는 어떤 모양이며, 모터에는 최대 몇 N·m가 필요할까?"

## 기능

- **입력**: 이동 거리 D (회전 rad / 직선 m 모드 전환), v_max, a_max, j_max, 프로파일 종류(저크 제한 S-커브 / 사다리꼴).
  부하 추정용으로 관성 J(kg·m²) 또는 질량 m(kg), 일정 부하 토크·힘(중력·마찰), 감속비 N, 효율 η, 모터 정격 토크.
- **프리셋** (편집 가능): "휴머노이드 무릎 관절 (일반적인 값)", "3D 프린터 / 갠트리 직선 축 (일반적인 값)". 특정 제품 값이 아닌 일반적인 예시입니다.
- **프로파일 모델**: 정지→정지 이동의 시간 최적 7구간 저크 제한 프로파일. v_max 미도달(등속 구간 없음), a_max 미도달(삼각 가속도), 둘 다 미도달인 경우를 모두 처리합니다.
  비교용으로 사다리꼴(무한 저크) 프로파일을 함께 계산합니다. 각 구간을 닫힌 식(3차 다항식)으로 평가하며, 수치 적분은 쓰지 않습니다.
- **결과**: T_j, T_a, T_v, 총 시간, 실제 도달한 최대 v·a, 모터 최대·RMS 토크, 최대 기계 출력, 모터 최대 속도, 사다리꼴 대비 S-커브 시간 증가(%), RMS 토크 대 정격 토크 확인(✅/⚠️).
- **동기화**: 두 번째 축이 자기 거리를 같은 한계값으로 이동하되, 축 1과 동시에 끝나도록 시간 스케일링합니다(속도 /k, 가속도 /k², 저크 /k³).
  축 2가 축 1보다 느리면 압축하지 않고 병목이라고 표시합니다.
- **3D 뷰** (WebGL2): 회전 모드에서는 모터 하우징, 감속기 링, 링크와 엔드 이펙터, 직선 모드에서는 레일, 캐리지, 벨트를 그립니다.
  시작·목표 고스트, 속도 화살표, |τ_m|/최대값에 따른 모터 하우징 색을 표시합니다. 목표 고스트를 끌어 D를 바로 바꿀 수 있습니다.
- **2D 그래프**: p, v, a, j를 시간축으로 겹쳐 그립니다(S-커브는 실선, 사다리꼴은 점선). 현재 시각이 커서로 표시됩니다.
- 한국어 / English 전환(브라우저 언어 기본값), 모바일 지원, 시스템 다크 모드.

## 모델과 가정

- 정지 → 정지 이동(시작·끝 속도와 가속도 0). 가속과 감속은 대칭입니다.
- S-커브: `T = D/v + v/a + a/j` (v_max와 a_max 모두 도달하는 경우). 나머지 경우는 교과서의 표준 식을 씁니다.
- 부하는 강체이고 관성은 일정합니다. 모터 동특성, 전류·전압 제한, 백래시, 탄성은 다루지 않으며 모터 로터 관성도 포함하지 않습니다.
- 모터 측 토크: `τ_m(t) = (J·a(t) + τ_load) / (N·η)`, 모터 속도 `ω_m = N·v`.
  직선 모드에서 N의 단위는 rad/m(이동 1 m당 모터 회전각, 예: 벨트 풀리 반지름의 역수)이고, J 대신 질량 m, τ_load 대신 힘 F_load를 씁니다.
- RMS 토크는 이동 시간 동안 정확히 적분한 값입니다(구간마다 τ가 시간에 대해 선형이므로).

> ⚠️ **교육용 추정치입니다. 실제 로봇에 쓰기 전에 반드시 검증하세요.**

## 조작

| 동작 | 마우스 | 터치 |
|---|---|---|
| 회전 | 왼쪽 드래그 | 한 손가락 드래그 |
| 이동(팬) | 오른쪽 드래그 | 두 손가락 드래그 |
| 확대/축소 | 휠 | 핀치 |
| 시점 초기화 | 더블클릭 | — |
| D 설정 | 밝은 목표 고스트 드래그 | 목표 고스트 드래그 |

뷰 위 버튼: 고스트, 속도, 토크 색상, 축 2 표시 전환, 그리고 원근 / 위 / 측면 / 정면 시점.
재생: 재생/정지, 속도(0.1× / 0.25× / 1×, 기본 0.25×), 시간 슬라이더, 반복. HUD에 t, 구간(1-7), p, v, a, τ_m이 표시됩니다.

## 로컬 빌드

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked   # 또는 릴리스 바이너리 사용
cargo test                 # 코어 + 렌더러 수학 단위 테스트
trunk build --release      # dist/ 에 정적 사이트 생성 (경로 /s-curve/)
sh tools/privacy_check.sh  # "clean" 이 출력되어야 합니다
```

`trunk serve` 로 로컬에서 볼 수 있습니다(`http://localhost:8080/s-curve/`).

## 개인정보

모든 계산은 브라우저 안에서 이루어집니다. 분석·추적 도구, 외부 CDN·폰트, 런타임 네트워크 요청이 없으며, CSP로 외부 요청을 막습니다.

## 라이선스

MIT — [LICENSE](LICENSE) 참고. Copyright (c) s-curve contributors.

---

<a id="english"></a>

# s-curve (English)

An S-curve motion profile lab for robot joints and linear axes: a static web app (Rust → WebAssembly) that runs entirely in your browser.

> "My joint / axis must move this distance with these velocity, acceleration and jerk limits — how long does the move take, what do position, velocity, acceleration and jerk look like, and what peak torque / force does the motor need?"

## Features

- **Inputs**: move distance D (rad for rotary or m for linear; mode toggle), v_max, a_max, j_max, profile type (jerk-limited S-curve vs. trapezoidal).
  For the load estimate: inertia J (kg·m²) or mass m (kg), constant load torque / force (gravity, friction), gear ratio N, efficiency η, motor rated torque.
- **Presets** (editable): "Humanoid knee joint (typical)" and "3D-printer / gantry linear axis (typical)". Generic example values, not any specific product.
- **Profile model**: exact time-optimal 7-segment jerk-limited profile for rest-to-rest moves, including the degenerate cases
  (v_max not reached → no cruise; a_max not reached → triangular acceleration; both). The trapezoidal profile (infinite jerk) is computed for comparison.
  Every segment is evaluated in closed form (a cubic in time); there is no numerical integration.
- **Results**: T_j, T_a, T_v, total time, peak v and a actually reached, peak and RMS motor torque, peak mechanical power, peak motor speed,
  S-curve time penalty vs. trapezoid (%), and an RMS-torque check against the rated torque (✅/⚠️).
- **Sync**: a second axis moves its own distance with the same limits, time-scaled to finish together with axis 1 (velocity /k, acceleration /k², jerk /k³).
  If axis 2 is slower than axis 1, it is not compressed and is flagged as the bottleneck.
- **3D view** (WebGL2): a motor housing, gearbox ring, link and end-effector in rotary mode, or a rail, carriage and belt in linear mode.
  The view shows start and target ghosts, a velocity arrow, and a motor-housing tint based on |τ_m| / peak. Drag the target ghost to set D directly.
- **2D plots**: stacked p, v, a and j against time (S-curve solid, trapezoid dashed), with the current time as a cursor.
- Korean / English toggle (defaults to the browser language), mobile friendly, follows the system dark mode.

## Model and assumptions

- Rest-to-rest moves: zero velocity and acceleration at the start and end. Acceleration and deceleration are symmetric.
- S-curve: `T = D/v + v/a + a/j` when both v_max and a_max are reached; the other regimes use the standard textbook equations.
- Rigid load with constant inertia. The model leaves out motor dynamics, current and voltage limits, backlash, compliance, and the motor rotor inertia.
- Motor-side torque `τ_m(t) = (J·a(t) + τ_load) / (N·η)`, motor speed `ω_m = N·v`.
  In linear mode N is in rad/m: motor radians per metre of travel, e.g. 1 / pulley radius. Mass m replaces J, and force F_load replaces τ_load.
- The RMS torque is integrated exactly over the move, because τ is linear in time within each segment.

> ⚠️ **Educational estimates: validate before using on a real robot.**

## Controls

| Action | Mouse | Touch |
|---|---|---|
| Orbit | left drag | one-finger drag |
| Pan | right drag | two-finger drag |
| Zoom | wheel | pinch |
| Reset view | double-click | — |
| Set D | drag the bright target ghost | drag the target ghost |

Buttons over the view toggle the ghosts, the velocity arrow, the torque tint and axis 2, and switch between the Perspective / Top / Side / Front views.
The playback controls are play/pause, speed (0.1× / 0.25× / 1×, default 0.25×), a time slider and loop. The HUD shows t, the segment (1-7), p, v, a and τ_m.

## Local build

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked   # or use the release binary
cargo test                 # core + renderer math unit tests
trunk build --release      # static site in dist/ (served under /s-curve/)
sh tools/privacy_check.sh  # must print "clean"
```

`trunk serve` serves it locally at `http://localhost:8080/s-curve/`.

## Privacy

Everything is computed in your browser. There are no analytics or trackers, no external CDNs or fonts, and no network requests at runtime; the CSP blocks external requests.

## License

MIT — see [LICENSE](LICENSE). Copyright (c) s-curve contributors.
