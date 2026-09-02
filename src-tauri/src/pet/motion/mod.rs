//! 모션 도메인 — **동작 하나가 파일 하나**다. 매 틱 물리·진입·퇴장이 함께 있다.
//!
//! 여기 없는 것은 의도다. `pick_next`(추첨기)·`enter`·`clamp`·난수는 `pet`에,
//! 튜닝 값은 `pet/tuning.rs`에 남는다 — 확률이 흩어지면 서로의 비중이 안 보인다.
//!
//! 부모에게 여는 진입점은 `pub(in crate::pet)`다. 여기서 `super`는 `pet`이 아니라
//! `motion`이라 `pub(super)`로는 정작 부르는 쪽이 못 본다.

mod fishing;
mod ground;
mod freakout;
