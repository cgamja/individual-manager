//! 펭귄의 클릭 판정 영역 — 기준은 창이 아니라 **그림**이다.
//!
//! 치수는 SVG `viewBox` 단위로만 적고 `PET_SIZE` 비율로 픽셀을 낸다 (`PET_PAD_*`·
//! `PET_WINDOW_*`는 안 쓴다). 같은 수가 `src/assets/penguin/hit.ts`에도 있고
//! `src/pet/pet-css.test.ts`가 대조한다.
//!
//! 배경과 설계 근거: `MOTIONS.md` "클릭의 경계는 창이 아니라 히트 상자다",
//! `docs/solutions/best-practices/macos-click-through-is-per-window.md`.

use crate::pet::PET_SIZE;

/// `Penguin` SVG의 `viewBox` — `src/assets/penguin/index.tsx`와 같아야 한다.
pub const PET_VIEWBOX_W: f64 = 100.0;
pub const PET_VIEWBOX_H: f64 = 130.0;

/// 펭귄이 그려진 자리 — **viewBox 단위**. 실측 bbox를 바깥으로 반올림하고
/// 좌우를 중심에 대칭으로 맞췄다(`.pg-stage--flip`이 그림만 뒤집는다).
/// 낚싯대·방망이는 뺐다 — `base.css`의 `pointer-events` 목록과 같아야 한다.
pub const PET_HIT_L: f64 = 14.0;
pub const PET_HIT_T: f64 = 12.0;
pub const PET_HIT_R: f64 = 86.0;
pub const PET_HIT_B: f64 = 130.0;

/// 되돌리기를 미리 거는 여유 — `PET_SIZE` 비율. 커서가 그림에 닿기 **전에**
/// 창이 클릭을 도로 먹기 시작한다.
///
/// 없으면 커서가 펭귄에 올라온 뒤 한 틱(50ms) + 세터 한 프레임 동안 클릭이
/// 아래 앱으로 샌다 — 빠르게 움직여 누르면 그 안에 들어간다. 일찍 거는 쪽의
/// 대가는 통과 영역이 줄어드는 것뿐이라 방향이 분명하다.
pub const PET_HIT_ARM_RATIO: f64 = 0.1;

/// 히스테리시스 띠 — 웹뷰가 요청하는 경계와 Rust가 되돌리는 경계 사이의 간격.
/// 둘이 정확히 맞닿으면 그 위에서 요청과 되돌리기가 번갈아 일어난다.
pub const PET_HIT_HYSTERESIS_RATIO: f64 = 0.02;

/// 통과를 시작한 자리에서 이만큼 멀어지면 되돌린다 — `PET_SIZE` 비율.
///
/// **상자 판정과 무관한 두 번째 문이다.** 상자 판정은 커서를 논리 좌표로
/// 바꿔야 하고 그 변환은 배율에 기대는데, 이 문은 커서가 움직였다는 사실만 본다.
/// 되돌아갈 때 부르는 쪽이 요청까지 지우므로(아래 "요청은 걸쇠다") 한 번 열리면
/// 웹뷰가 자기 좌표로 다시 판단할 때까지 닫히지 않는다.
pub const PET_DRIFT_RATIO: f64 = 1.0;

const _: () = assert!(PET_HIT_L < PET_HIT_R);
const _: () = assert!(PET_HIT_T < PET_HIT_B);
const _: () = assert!(PET_HIT_L >= 0.0 && PET_HIT_R <= PET_VIEWBOX_W);
const _: () = assert!(PET_HIT_T >= 0.0 && PET_HIT_B <= PET_VIEWBOX_H);
/// 뒤집어도 같은 상자여야 한다.
const _: () = assert!(PET_HIT_L + PET_HIT_R == PET_VIEWBOX_W);
/// 0이면 경계에서 진동하고, 너무 넓으면 여백을 도로 삼킨다.
const _: () = assert!(PET_HIT_HYSTERESIS_RATIO > 0.0);
const _: () = assert!(PET_HIT_ARM_RATIO > PET_HIT_HYSTERESIS_RATIO);
const _: () = assert!(PET_HIT_ARM_RATIO < 0.5);
const _: () = assert!(PET_DRIFT_RATIO > 0.0);

/// `(left, top, right, bottom)`. 좌표계는 부르는 쪽이 정한다.
pub type Rect = (f64, f64, f64, f64);

/// viewBox 한 단위가 몇 px인가. `preserveAspectRatio`의 기본값 `xMidYMid meet`
/// 이라 **짧은 쪽 배율**이 이긴다.
fn art_scale() -> f64 {
    let sx = PET_SIZE / PET_VIEWBOX_W;
    let sy = PET_SIZE / PET_VIEWBOX_H;
    if sx < sy {
        sx
    } else {
        sy
    }
}

/// 무대(한 변 `PET_SIZE`) 안에서 그림이 시작하는 자리 — `meet`의 레터박스.
fn art_origin() -> (f64, f64) {
    let s = art_scale();
    (
        (PET_SIZE - PET_VIEWBOX_W * s) / 2.0,
        (PET_SIZE - PET_VIEWBOX_H * s) / 2.0,
    )
}

/// 펭귄이 그려진 자리. `pet_x`/`pet_y`는 무대 좌상단(= 스냅샷의 `x`/`y`).
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

/// Rust가 통과를 되돌리는 상자 — [`hit_rect`]에 여유를 더한 것.
pub fn revert_rect(pet_x: f64, pet_y: f64) -> Rect {
    inflate(hit_rect(pet_x, pet_y), PET_SIZE * PET_HIT_ARM_RATIO)
}

/// 웹뷰가 통과를 요청해도 되는 상자 — [`revert_rect`] 밖이어야 한다.
pub fn request_rect(pet_x: f64, pet_y: f64) -> Rect {
    inflate(
        revert_rect(pet_x, pet_y),
        PET_SIZE * PET_HIT_HYSTERESIS_RATIO,
    )
}

pub fn inflate((l, t, r, b): Rect, by: f64) -> Rect {
    (l - by, t - by, r + by, b + by)
}

/// 반열린 구간 — 오른쪽·아래 변은 밖으로 친다.
pub fn contains((l, t, r, b): Rect, x: f64, y: f64) -> bool {
    x >= l && x < r && y >= t && y < b
}

/// 그림이 지금 상자 안에 들어와 있는 자세인가.
///
/// 상자는 **쉬는 자세**의 bbox다. 슬라이딩·굴러떨어지기·오르내리는 헤엄·
/// 널브러짐·던져짐은 그림을 상자 밖으로 수십 px 내보내므로, 그 동안 통과를
/// 걸면 그려진 펭귄을 눌렀는데 아래 앱이 받는다. 그때는 통과를 접는다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose {
    /// 걷기·유휴·잠·낚시·반응처럼 서 있는 국면.
    InBox,
    /// 회전·납작해짐으로 상자를 넘어가거나, 들려 있는 국면(R4).
    OutOfBox,
}

impl Pose {
    pub fn in_box(self) -> bool {
        self == Pose::InBox
    }
}

/// 이번 틱에 이 창이 클릭을 통과시켜야 하는가. 좌표는 화면 **논리** 좌표다.
///
/// **요청은 걸쇠다** — 거짓이 나오면 부르는 쪽([`super::apply_click_through`])이
/// 요청까지 지운다. 그래야 통과가 "요청이 남아 있는 동안"이 아니라 "근거가
/// 있는 동안"만 유지되고, 되돌린 뒤에는 웹뷰가 **자기 client 좌표로** 다시
/// 판단해야 켜진다 — 배율이 틀려도 되찾을 수 있는 길이 그것뿐이다.
///
/// 참으로 가는 길은 하나, 거짓으로 가는 길이 다섯이다. 거짓은 곧 오늘까지의
/// 동작(창이 클릭을 먹는다)이라 무엇이 어긋나든 최악이 회귀 없음이다.
pub fn decide_click_through(
    requested: bool,
    pose: Pose,
    pet: (f64, f64),
    cursor: Option<(f64, f64)>,
    anchor: Option<(f64, f64)>,
) -> bool {
    if !requested {
        return false;
    }
    if !pose.in_box() {
        return false;
    }
    // 커서나 배율을 못 읽었다.
    let Some((cx, cy)) = cursor else {
        return false;
    };
    // 커서가 그림 근처로 돌아왔다.
    if contains(revert_rect(pet.0, pet.1), cx, cy) {
        return false;
    }
    // 요청받은 자리에서 한 마리 폭 넘게 움직였다.
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
