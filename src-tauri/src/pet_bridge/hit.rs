//! 펭귄의 클릭 판정 영역 — 기준은 **창이 아니라 그림**이다.
//!
//! 창은 `PET_SIZE`짜리 정사각 무대에 방망이·말풍선 여백까지 더한 244×220이지만,
//! 펭귄이 실제로 그려진 자리는 그 17%뿐이다. 나머지는 투명한데도 macOS의 히트
//! 테스트가 알파를 안 보므로 창이 통째로 클릭을 먹는다.
//!
//! **치수는 SVG `viewBox` 단위로 한 번만 적고 `PET_SIZE` 비율로 픽셀을 낸다.**
//! `PET_PAD_*`도 `PET_WINDOW_*`도 이 파일의 계산에 안 들어간다 — 여백이 바뀌어도
//! 판정은 무관해야 하고, 크기가 바뀌면 판정이 따라가야 한다.
//!
//! 같은 수가 `src/assets/penguin/hit.ts`에도 있다 (CSS·TS는 Rust를 못 부른다).
//! `src/pet/pet-css.test.ts`의 "히트 박스 상수 동기화"가 둘을 대조한다.

use crate::pet::PET_SIZE;

/// `Penguin` SVG의 `viewBox` — `src/assets/penguin/index.tsx`의 `0 0 100 130`.
/// 이 값이 바뀌면 아래 상자가 통째로 어긋난다.
pub const PET_VIEWBOX_W: f64 = 100.0;
pub const PET_VIEWBOX_H: f64 = 130.0;

/// 펭귄 실루엣을 감싸는 상자 — **viewBox 단위**다.
///
/// 실측 bbox는 x 14.7..81.3, y 12.7..128.8이다 (꼬리 끝 16, 부리 끝 77, 가까운쪽
/// 날개 80, 머리 위 14, 그림자 아래 127.5에 후광 stroke 절반 1.3을 더한 값).
/// 바깥으로 반올림해 관용을 1단위쯤 남겼다.
///
/// **낚싯대(x 98)와 방망이는 일부러 뺐다** — 둘 다 웹뷰에서도 클릭을 안 받는다
/// (`base.css`의 `pointer-events: none`). 한쪽만 넣으면 "웹뷰는 반응하는데 창은
/// 통과시키는" 갈래가 생긴다.
pub const PET_HIT_L: f64 = 14.0;
pub const PET_HIT_T: f64 = 12.0;
pub const PET_HIT_R: f64 = 82.0;
pub const PET_HIT_B: f64 = 130.0;

/// 히스테리시스 띠의 폭 — `PET_SIZE`에 대한 비율.
///
/// 웹뷰는 [`request_rect`] **밖**에서만 통과를 요청하고, Rust는 [`hit_rect`]
/// **안**에서 되돌린다. 두 상자 사이의 띠에서는 어느 전이도 안 일어나므로
/// 경계에서 켰다 껐다 하지 않는다.
///
/// **띠는 안전한 쪽(클릭을 먹는 쪽)으로 치우쳐 있다.** 두 판정이 쓰는 좌표가
/// 다르기 때문이다 — 웹뷰는 client 좌표, Rust는 스냅샷 + 물리 커서다. 반올림과
/// 한 틱치 창 이동만큼 어긋날 수 있고, 그 어긋남이 "펭귄 위인데 통과 중"으로
/// 기울면 안 된다.
pub const PET_HIT_HYSTERESIS_RATIO: f64 = 0.02;

const _: () = assert!(PET_HIT_L < PET_HIT_R);
const _: () = assert!(PET_HIT_T < PET_HIT_B);
const _: () = assert!(PET_HIT_L >= 0.0 && PET_HIT_R <= PET_VIEWBOX_W);
const _: () = assert!(PET_HIT_T >= 0.0 && PET_HIT_B <= PET_VIEWBOX_H);
/// 0이면 히스테리시스가 없어져 경계에서 진동한다.
const _: () = assert!(PET_HIT_HYSTERESIS_RATIO > 0.0);
/// 너무 넓으면 띠가 여백을 다시 삼켜 이 작업의 목적이 사라진다.
const _: () = assert!(PET_HIT_HYSTERESIS_RATIO < 0.1);

/// `(left, top, right, bottom)`. 좌표계는 부르는 쪽이 정한다 — 화면 논리 좌표든
/// 창 안 client 좌표든 같은 함수를 쓴다.
pub type Rect = (f64, f64, f64, f64);

/// viewBox 한 단위가 몇 px인가.
///
/// `preserveAspectRatio`의 기본값 `xMidYMid meet`이라 **짧은 쪽 배율**이 이긴다.
/// 지금은 세로(130)가 커서 세로가 꽉 차지만, viewBox가 바뀌면 반대가 될 수 있어
/// `min`을 그대로 적는다.
fn art_scale() -> f64 {
    let sx = PET_SIZE / PET_VIEWBOX_W;
    let sy = PET_SIZE / PET_VIEWBOX_H;
    if sx < sy {
        sx
    } else {
        sy
    }
}

/// 무대(한 변 `PET_SIZE`) 안에서 그림이 시작하는 자리. `meet`이 남긴 레터박스다.
fn art_origin() -> (f64, f64) {
    let s = art_scale();
    (
        (PET_SIZE - PET_VIEWBOX_W * s) / 2.0,
        (PET_SIZE - PET_VIEWBOX_H * s) / 2.0,
    )
}

/// 펭귄이 실제로 그려진 자리. `pet_x`/`pet_y`는 무대의 좌상단(= 코어 스냅샷의
/// `x`/`y`)이다.
///
/// **Rust가 통과를 되돌리는 기준이 이 상자다.** 여기 들어오면 창은 다시 클릭을
/// 먹는다.
pub fn hit_rect(pet_x: f64, pet_y: f64) -> Rect {
    let s = art_scale();
    let (ox, oy) = art_origin();
    (
        pet_x + ox + PET_HIT_L * s,
        pet_y + oy + PET_HIT_T * s,
        pet_x + ox + PET_HIT_R * s,
        pet_y + oy + PET_HIT_B * s,
    )
}

/// 웹뷰가 "통과시켜 달라"고 말해도 되는 바깥 상자 — [`hit_rect`]를 히스테리시스
/// 만큼 부풀린 것이다.
pub fn request_rect(pet_x: f64, pet_y: f64) -> Rect {
    inflate(
        hit_rect(pet_x, pet_y),
        PET_SIZE * PET_HIT_HYSTERESIS_RATIO,
    )
}

pub fn inflate((l, t, r, b): Rect, by: f64) -> Rect {
    (l - by, t - by, r + by, b + by)
}

/// 반열린 구간이다 — 오른쪽·아래 변은 밖으로 친다. 두 상자가 맞닿았을 때 같은
/// 점이 양쪽에 들지 않게 한다.
pub fn contains((l, t, r, b): Rect, x: f64, y: f64) -> bool {
    x >= l && x < r && y >= t && y < b
}

/// 통과를 시작한 자리에서 이만큼 멀어지면 무조건 되돌린다 — `PET_SIZE` 배수.
///
/// **상자 판정과 무관한 두 번째 문이다.** 상자 판정은 커서를 논리 좌표로
/// 바꿔야 하고 그 변환은 배율에 기댄다. 배율이 틀리면 상자 판정이 영영
/// 안 맞아 창이 통과 상태로 굳는데, 그게 "펭귄을 아예 못 누른다"로 가는
/// 유일한 길이다. 이 문은 **커서가 실제로 움직였다**는 사실만 보므로 배율이
/// 어긋나도 결국 열린다 (어긋난 배율은 거리도 같은 비율로 늘리거나 줄일 뿐이다).
///
/// 한 마리 폭만큼이다 — 더 짧으면 손떨림에 풀리고, 더 길면 복구가 늦다.
pub const PET_DRIFT_RATIO: f64 = 1.0;
const _: () = assert!(PET_DRIFT_RATIO > 0.0);

/// 이번 틱에 이 창이 클릭을 통과시켜야 하는가.
///
/// **참으로 가는 길은 하나뿐이고 거짓으로 가는 길이 다섯이다.** 거짓이
/// "클릭을 먹는다"= 오늘까지의 동작이므로, 무엇이 어긋나든 최악은 회귀 없음이다
/// (R6). 좌표는 전부 화면 **논리** 좌표다.
pub fn decide_click_through(
    requested: bool,
    dragged: bool,
    pet: (f64, f64),
    cursor: Option<(f64, f64)>,
    anchor: Option<(f64, f64)>,
) -> bool {
    // 1) 웹뷰가 요청하지 않았다. 통과는 근거가 있을 때만 켜진다.
    if !requested {
        return false;
    }
    // 2) 들고 있다 — 커서가 창 어디로 가든 드래그가 끊기면 안 된다 (R4).
    if dragged {
        return false;
    }
    // 3) 커서를 못 읽었다(또는 배율을 못 읽어 부르는 쪽이 `None`을 줬다).
    let Some((cx, cy)) = cursor else {
        return false;
    };
    // 4) 커서가 펭귄 위로 돌아왔다.
    if contains(hit_rect(pet.0, pet.1), cx, cy) {
        return false;
    }
    // 5) 요청받은 자리에서 한 마리 폭 넘게 움직였다 — 배율과 무관한 두 번째 문.
    if let Some((ax, ay)) = anchor {
        if (cx - ax).hypot(cy - ay) > PET_SIZE * PET_DRIFT_RATIO {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[path = "hit_tests.rs"]
mod tests;
