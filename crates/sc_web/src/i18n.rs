//! English / Korean UI strings. Keys ending in ".lin" override the plain key in linear mode.

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Ko,
}

const T: &[(&str, &str, &str)] = &[
    ("title", "S-curve motion profile lab", "S-커브 모션 프로파일 랩"),
    ("subtitle", "How long does the move take, and how much torque does the motor need?", "이 이동에 몇 초가 걸리고, 모터 토크는 얼마나 필요할까?"),
    ("preset", "Preset", "프리셋"),
    ("preset_custom", "Custom", "사용자 정의"),
    ("preset_knee", "Humanoid knee joint (typical)", "휴머노이드 무릎 관절 (일반적인 값)"),
    ("preset_gantry", "3D-printer / gantry linear axis (typical)", "3D 프린터 / 갠트리 직선 축 (일반적인 값)"),
    ("sec_move", "Move", "이동"),
    ("mode", "Mode", "모드"),
    ("mode_rot", "Rotary joint", "회전 관절"),
    ("mode_lin", "Linear axis", "직선 축"),
    ("kind", "Animated profile", "애니메이션 프로파일"),
    ("kind_s", "Jerk-limited S-curve", "저크 제한 S-커브"),
    ("kind_t", "Trapezoidal", "사다리꼴"),
    ("d", "Distance D", "이동 거리 D"),
    ("v", "Max velocity v_max", "최대 속도 v_max"),
    ("a", "Max acceleration a_max", "최대 가속도 a_max"),
    ("j", "Max jerk j_max", "최대 저크 j_max"),
    ("rest", "Rest-to-rest move (starts and ends at zero velocity and acceleration).", "정지 → 정지 이동 (시작·끝 속도와 가속도 0)."),
    ("sec_load", "Load", "부하"),
    ("inertia", "Load inertia J", "부하 관성 J"),
    ("inertia.lin", "Moving mass m", "이동 질량 m"),
    ("load", "Constant load torque (gravity, friction)", "일정 부하 토크 (중력·마찰)"),
    ("load.lin", "Constant load force (gravity, friction)", "일정 부하 힘 (중력·마찰)"),
    ("ratio", "Gear ratio N", "감속비 N"),
    ("ratio.lin", "Transmission N (motor rad per m)", "전달비 N (m당 모터 rad)"),
    ("eff", "Gear efficiency η", "감속기 효율 η"),
    ("eff.lin", "Transmission efficiency η", "전달 효율 η"),
    ("rated", "Motor rated torque", "모터 정격 토크"),
    ("sec_sync", "Sync", "동기화"),
    ("sync", "Second axis", "두 번째 축"),
    ("d2", "Axis 2 distance", "축 2 이동 거리"),
    ("sync_note", "Same limits; time-scaled to finish together with axis 1.", "같은 한계값을 쓰고, 축 1과 동시에 끝나도록 시간 스케일링합니다."),
    ("tg_ghost", "Ghosts", "고스트"),
    ("tg_vel", "Velocity", "속도"),
    ("tg_tint", "Torque tint", "토크 색상"),
    ("tg_axis2", "Axis 2", "축 2"),
    ("vp_persp", "Persp.", "원근"),
    ("vp_top", "Top", "위"),
    ("vp_side", "Side", "측면"),
    ("vp_front", "Front", "정면"),
    ("play", "Play", "재생"),
    ("pause", "Pause", "정지"),
    ("loop", "Loop", "반복"),
    ("speed", "Speed", "속도"),
    ("nogl", "WebGL2 is not available: the 3D view is off, the plots still work.", "WebGL2를 사용할 수 없어 3D 뷰를 끕니다. 그래프는 계속 동작합니다."),
    ("hint", "Drag: orbit · right-drag / two fingers: pan · wheel / pinch: zoom · double-click: reset · drag the bright target ghost to set D", "드래그: 회전 · 우클릭 드래그 / 두 손가락: 이동 · 휠 / 핀치: 확대 · 더블클릭: 초기화 · 밝은 목표 고스트를 끌어 D 설정"),
    ("sec_results", "Results", "결과"),
    ("sec_plots", "Profiles", "프로파일 그래프"),
    ("col_s", "S-curve", "S-커브"),
    ("col_t", "Trapezoid", "사다리꼴"),
    ("r_total", "Total time", "총 이동 시간"),
    ("r_tj", "T_j (jerk phase)", "T_j (저크 구간)"),
    ("r_ta", "T_a (accel phase)", "T_a (가속 구간)"),
    ("r_tv", "T_v (cruise)", "T_v (등속 구간)"),
    ("r_vpk", "Peak v reached", "도달 최대 속도"),
    ("r_apk", "Peak a reached", "도달 최대 가속도"),
    ("r_tpk", "Peak motor torque", "모터 최대 토크"),
    ("r_trms", "RMS motor torque", "모터 RMS 토크"),
    ("r_pow", "Peak mech. power", "최대 기계 출력"),
    ("r_spd", "Peak motor speed", "모터 최대 속도"),
    ("r_pen", "S-curve time penalty vs trapezoid", "사다리꼴 대비 S-커브 시간 증가"),
    ("r_rated", "RMS torque vs rated", "RMS 토크 / 정격"),
    ("r_ok", "within rated", "정격 이내"),
    ("r_over", "exceeds rated", "정격 초과"),
    ("r_sync", "Axis 2", "축 2"),
    ("r_sync_ok", "stretched from {n} s to {t} s, peak v {v}", "{n} s → {t} s로 늘림, 최대 속도 {v}"),
    ("r_sync_slow", "needs {n} s > axis 1 {t} s: axis 2 is the bottleneck (not compressed)", "{n} s 필요 > 축 1 {t} s: 축 2가 병목 (압축하지 않음)"),
    ("note_j", "trapezoid jerk: impulses (not drawn)", "사다리꼴 저크: 임펄스 (표시 안 함)"),
    ("err", "Check the inputs: limits, ratio and efficiency must be positive numbers (η ≤ 1).", "입력값을 확인하세요: 한계값, 감속비, 효율은 양수여야 합니다 (η ≤ 1)."),
    ("disclaimer", "Educational estimates — validate before using on a real robot. Assumes a rest-to-rest move, rigid load, constant inertia, no motor dynamics or current limits; motor rotor inertia is not included.", "교육용 추정치입니다. 실제 로봇에 쓰기 전에 반드시 검증하세요. 정지→정지 이동, 강체 부하, 일정 관성, 모터 동특성·전류 제한 없음을 가정하며 모터 로터 관성은 포함하지 않습니다."),
    ("footer", "MIT license · runs entirely in your browser, nothing is sent anywhere", "MIT 라이선스 · 브라우저 안에서만 실행되며 아무것도 전송하지 않습니다"),
    ("lang", "한국어", "English"),
];

pub fn tr(lang: Lang, key: &str, linear: bool) -> &'static str {
    let find = |k: &str| T.iter().find(|e| e.0 == k).map(|e| if lang == Lang::Ko { e.2 } else { e.1 });
    let lin = if linear { find(&format!("{key}.lin")) } else { None };
    lin.or_else(|| find(key)).unwrap_or("?")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_unique_and_linear_override() {
        for (i, a) in T.iter().enumerate() {
            assert!(T[i + 1..].iter().all(|b| b.0 != a.0), "duplicate {}", a.0);
        }
        assert_eq!(tr(Lang::En, "inertia", true), "Moving mass m");
        assert_eq!(tr(Lang::Ko, "inertia", false), "부하 관성 J");
        assert_eq!(tr(Lang::En, "missing", false), "?");
    }
}
