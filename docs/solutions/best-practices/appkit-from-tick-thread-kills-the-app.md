# AppKit을 틱 스레드에서 만지면 앱이 흔적 없이 죽는다

## 증상

설정 창에서 **비치발리볼**을 누르면 앱이 통째로 사라졌다. 펭귄 창도, 트레이 아이콘도,
설정 창도 함께 없어졌다.

찾기 어려웠던 이유는 **아무 단서도 안 남기 때문**이다:

- **패닉이 아니다.** 백트레이스도, `panicked at`도, 어떤 에러 메시지도 없다.
- **Tauri의 종료 이벤트도 안 뜬다.** `RunEvent::ExitRequested`도 `RunEvent::Exit`도
  찍히지 않는다 — 즉 "창이 다 닫혀서 앱이 끝난" 정상 경로가 **아니다.**
- 마지막 로그가 `start_volleyball -> true`다. **"판을 잘 열었다"고 말한 직후에**
  프로세스가 증발한다.
- `cargo test`(416개), `npm test`, `npm run build`, 번들 빌드가 **전부 통과한다.**

## 원인

코트 창의 레벨을 내리는 `sink_court_below_pets`가 **20Hz 틱 스레드에서** AppKit을
직접 만졌다.

```rust
// tick.rs: 틱 스레드 → apply_volley → create_court_window → sink_court_below_pets
let ptr = window.ns_window()?;
unsafe {
    let ns = &*(ptr as *const NSWindow);
    ns.setLevel(COURT_WINDOW_LEVEL);   // ← 메인 스레드가 아니다
}
```

macOS에서 AppKit 객체는 메인 스레드에서만 만질 수 있다. 위반하면 예외를 던지는 대신
프로세스가 죽는다.

**핀볼 판은 같은 함수를 갖고 있는데 왜 멀쩡했나.** 호출 자리가 다르다 —
`sink_pinball_below_pets`는 **커맨드**(`commands.rs`, `window.rs`)에서 불리고,
`sink_court_below_pets`는 **틱 스레드**에서 불린다. 코트 창을 만드는 주체가
커맨드가 아니라 틱(`apply_volley`)이라서 생긴 차이다. 코드를 베껴 오면서 함수는
같이 왔지만 **그 함수가 안전했던 이유(호출 스레드)는 안 따라왔다.**

## 안 통한 시도

- **로그를 더 봤다.** `npm run tauri dev`의 stdout/stderr에는 아무것도 없다.
  `EXIT=0`만 남아서 "정상 종료"로 오해했다 — 그래서 한동안 "창이 다 닫혀서
  Tauri가 앱을 끝냈나"를 팠다. **`RunEvent`를 전부 찍어 보고서야** 그 경로가
  아님이 드러났다(`ExitRequested`가 한 번도 안 뜬다).
- **`RUST_BACKTRACE=1`.** 패닉이 아니므로 아무 효과가 없다.
- **창 생성 실패를 의심했다.** `create_court_window`의 실패 갈래를 다 읽었지만
  실제로는 창이 성공적으로 만들어진 **뒤에** 죽는다.

결정적이었던 것은 **사용자 클릭 없이 재현되게 만든 것**이다. `setup()`에 6초 뒤
`start_volleyball`을 부르는 스레드를 심어 놓으니 매번 같은 자리에서 죽었고,
그 자리에서 아래로 한 겹씩 내려가 AppKit 호출에 닿았다.

## 해결

AppKit 호출을 `run_on_main_thread`로 감싼다.

```rust
let app = app.clone();
let _ = app.clone().run_on_main_thread(move || {
    // ns_window() + setLevel: 은 전부 이 안에서
});
```

## 왜 통하는가

`run_on_main_thread`는 클로저를 이벤트 루프에 넘겨 메인 스레드에서 실행시킨다.
AppKit의 요구를 그대로 만족시키므로 어느 스레드에서 불러도 안전해진다.

## 예방책

**`CLAUDE.md`의 KTD5를 반만 읽으면 정확히 이 함정에 빠진다.** 거기엔
*"`set_position`은 어느 스레드에서 불러도 안전하다 — tauri-runtime-wry가 이벤트
루프로 넘기고 tao의 macOS 구현이 다시 메인 스레드로 디스패치한다"*고 적혀 있다.
맞는 말이지만 **Tauri API에 한한 이야기**다. `ns_window()`로 포인터를 꺼내 직접
만지는 것은 **그 디스패치를 건너뛴다.**

규칙으로 적으면:

> **Tauri API는 스레드를 안 가려도 되지만, `ns_window()` 아래로 내려간 순간부터는
> 반드시 메인 스레드다.**

그리고 이 결함군을 진단하는 법 하나 — **패닉도 종료 이벤트도 없이 앱이 사라지면
AppKit을 백그라운드 스레드에서 만진 곳을 찾는다.** 진단의 첫 수는 `.run(|_, event|)`에
`RunEvent`를 전부 찍어 보는 것이다. 종료 이벤트가 **안 뜨는 것**이 곧 "정상 종료가
아니다"라는 증거다.
