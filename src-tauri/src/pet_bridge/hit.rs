//! 펭귄의 클릭 판정 영역 — 기준은 창이 아니라 **그림**이다.
//!
//! 치수는 SVG `viewBox` 단위로만 적고 **화면에 그려지는 크기**(`pet_render_px`)
//! 비율로 픽셀을 낸다 (`PET_PAD_*`·`PET_WINDOW_*`는 안 쓴다). 같은 수가
//! `src/assets/penguin/hit.ts`에도 있고 `src/pet/pet-css.test.ts`가 대조한다.
//!
//! **좌표는 전부 화면 논리 px다.** 코어가 주는 펭귄 좌표는 "펭귄이 늘 `PET_SIZE`인
//! 세계"의 값이라 여기 들어올 때 `to_screen`을 지난다 (플랜 026 KTD1).
//!
//! 배경과 설계 근거: `MOTIONS.md` "클릭의 경계는 창이 아니라 히트 상자다",
//! `docs/solutions/best-practices/macos-click-through-is-per-window.md`.

use super::{pet_render_px, to_screen};

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

/// 되돌리기를 미리 거는 여유 — **화면에 그려지는 크기**의 비율. 커서가 그림에
/// 닿기 **전에** 창이 클릭을 도로 먹기 시작한다.
///
/// **절대 픽셀이 아니라 비율로 둔다.** 크기를 60%로 줄이면 여유도 14px → 8.4px로
/// 줄지만, 상자를 좁히는 쪽(펭귄이 커서로 걸어오는 속도)도 같은 배율로 줄어
/// **여유 ÷ 한 틱 이동량은 안 변한다**(균일 축소, 플랜 026 KTD2). 커서가 다가오는
/// 속도만 배율을 안 타는데, 그쪽은 원래 이 여유로 못 막는 크기이고
/// (빠른 마우스는 100%에서도 한 틱에 14px을 넘는다) 대신 [`PET_DRIFT_RATIO`]가
/// 배율만큼 **좁아져** 더 빨리 걸린다. 절대값으로 바꾸면 작은 펭귄에서 여유가
/// 상대적으로 커져 통과 영역을 도로 삼킨다.
///
/// 없으면 커서가 펭귄에 올라온 뒤 한 틱(50ms) + 세터 한 프레임 동안 클릭이
/// 아래 앱으로 샌다 — 빠르게 움직여 누르면 그 안에 들어간다. 일찍 거는 쪽의
/// 대가는 통과 영역이 줄어드는 것뿐이라 방향이 분명하다.
pub const PET_HIT_ARM_RATIO: f64 = 0.1;

/// 히스테리시스 띠 — 웹뷰가 요청하는 경계와 Rust가 되돌리는 경계 사이의 간격.
/// 둘이 정확히 맞닿으면 그 위에서 요청과 되돌리기가 번갈아 일어난다.
pub const PET_HIT_HYSTERESIS_RATIO: f64 = 0.02;

/// 통과를 시작한 자리에서 이만큼 멀어지면 되돌린다 — 화면에 그려지는 크기의 비율.
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
fn art_scale(scale: f64) -> f64 {
    let px = pet_render_px(scale);
    let sx = px / PET_VIEWBOX_W;
    let sy = px / PET_VIEWBOX_H;
    if sx < sy {
        sx
    } else {
        sy
    }
}

/// 무대(한 변 `pet_render_px`) 안에서 그림이 시작하는 자리 — `meet`의 레터박스.
fn art_origin(scale: f64) -> (f64, f64) {
    let s = art_scale(scale);
    let px = pet_render_px(scale);
    ((px - PET_VIEWBOX_W * s) / 2.0, (px - PET_VIEWBOX_H * s) / 2.0)
}

/// 펭귄이 그려진 자리 (화면 논리 px). `pet_x`/`pet_y`는 무대 좌상단이고
/// **코어 좌표**다(= 스냅샷의 `x`/`y`) — 여기서 화면으로 옮긴다.
pub fn hit_rect(pet_x: f64, pet_y: f64, scale: f64) -> Rect {
    let s = art_scale(scale);
    let (ox, oy) = art_origin(scale);
    let (x, y) = (to_screen(pet_x, scale), to_screen(pet_y, scale));
    (
        x + ox + PET_HIT_L * s,
        y + oy + PET_HIT_T * s,
        x + ox + PET_HIT_R * s,
        y + oy + PET_HIT_B * s,
    )
}

/// Rust가 통과를 되돌리는 상자 — [`hit_rect`]에 여유를 더한 것.
pub fn revert_rect(pet_x: f64, pet_y: f64, scale: f64) -> Rect {
    inflate(
        hit_rect(pet_x, pet_y, scale),
        pet_render_px(scale) * PET_HIT_ARM_RATIO,
    )
}

/// 웹뷰가 통과를 요청해도 되는 상자 — [`revert_rect`] 밖이어야 한다.
pub fn request_rect(pet_x: f64, pet_y: f64, scale: f64) -> Rect {
    inflate(
        revert_rect(pet_x, pet_y, scale),
        pet_render_px(scale) * PET_HIT_HYSTERESIS_RATIO,
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

/// 이번 틱의 판정. **접는 이유를 둘로 가른다** — 요청을 지울지가 갈리기 때문이다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// 통과시킨다.
    Through,
    /// 이번 틱만 접는다. **요청은 남긴다** — 자세나 커서 읽기처럼 곧 지나가는
    /// 사정이고, 지우면 사용자는 마우스를 다시 흔들어야 한다(웹뷰는 포인터가
    /// 움직일 때만, 그것도 스로틀에 걸려 재요청한다).
    Hold,
    /// 되돌리고 **요청도 지운다.** 커서가 그림 근처로 왔거나 너무 멀어졌다 —
    /// 둘 다 배율에 기대는 판정이라, 다시 켜려면 웹뷰의 client 좌표를 거쳐야
    /// 한다. 배율이 틀려도 되찾을 수 있는 길이 그것뿐이다.
    Latch,
}

impl Verdict {
    pub fn through(self) -> bool {
        self == Verdict::Through
    }
    pub fn latches(self) -> bool {
        self == Verdict::Latch
    }
}

/// 이번 틱에 이 창이 클릭을 통과시켜야 하는가. 좌표는 화면 **논리** 좌표다.
///
/// 통과로 가는 길은 하나, 접는 길이 다섯이다. 접는 것은 곧 오늘까지의 동작
/// (창이 클릭을 먹는다)이라 무엇이 어긋나든 최악이 회귀 없음이다.
pub fn decide_click_through(
    requested: bool,
    pose: Pose,
    pet: (f64, f64),
    cursor: Option<(f64, f64)>,
    anchor: Option<(f64, f64)>,
    scale: f64,
) -> Verdict {
    if !requested {
        return Verdict::Hold;
    }
    if !pose.in_box() {
        return Verdict::Hold;
    }
    // 커서나 배율을 못 읽었다.
    let Some((cx, cy)) = cursor else {
        return Verdict::Hold;
    };
    // 커서가 그림 근처로 돌아왔다.
    if contains(revert_rect(pet.0, pet.1, scale), cx, cy) {
        return Verdict::Latch;
    }
    // 요청받은 자리에서 한 마리 폭 넘게 움직였다.
    if let Some((ax, ay)) = anchor {
        if (cx - ax).hypot(cy - ay) > pet_render_px(scale) * PET_DRIFT_RATIO {
            return Verdict::Latch;
        }
    }
    Verdict::Through
}

#[cfg(test)]
#[path = "hit_tests.rs"]
mod tests;
