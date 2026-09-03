import { useEffect, useState } from "react";
import { Penguin } from "../assets/penguin";
import { getQueenState, onQueenState, type QueenSnapshot } from "../lib/pet";

/**
 * 미녀 펭귄 창 — 챔피언에게 챔피언 벨트를 채워 주는 배우.
 *
 * **`Pets`의 일원이 아니다.** 마릿수·삭제 대상·시드·저장에 걸리면 안 되므로
 * 가짜 `Pet`을 만들지 않고 자기 창을 갖는다. 그림은 `<Penguin>`을 **그대로**
 * 재사용한다 — 바닐라 SVG로 새로 그리면 펭귄 그림이 두 벌이 되고, 그걸 없앤 것이
 * 에셋 리팩터링(#54)이었다.
 *
 * **`PetApp`과 나눈 이유**: 저쪽은 드래그·히트박스·소리·말풍선을 다 갖는데
 * 여기서는 그 전부가 죽은 코드다. 이 파일이 하는 일은 국면에 따라 클래스를
 * 바꾸는 것뿐이고, **자리는 Rust 틱이 창을 옮겨서 정한다.**
 *
 * 화장(`pg-glam`)은 이 창에서만 켜고 훌라 차림은 안 입는다 — 저쪽은 비치발리볼
 * 판의 옷이라, 야차에 나오면 두 판이 섞인다.
 */
export function QueenApp() {
  const [queen, setQueen] = useState<QueenSnapshot | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    // **받는 쪽을 창에 묶는다.** 전역 `listen()`은 대상을 `Any`로 등록해서
    // emit 대상과 무관하게 전부 호출된다
    // (`docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`).
    void onQueenState(setQueen)
      .then((un) => {
        unlisten = un;
      })
      .catch(() => {});
    // **첫 상태는 구독으로 안 온다.** 틱이 이 창을 만들고 **같은 호출에서**
    // 보내므로 위 리스너가 붙기 전에 지나간다 — 안 받아 오면 걸어 들어오는
    // 국면을 통째로 놓치고 갑자기 챔피언 옆에 나타난다.
    void getQueenState()
      .then((q) => {
        if (q) setQueen((prev) => prev ?? q);
      })
      .catch(() => {});
    return () => unlisten?.();
  }, []);

  const pose = queen?.pose ?? "walk_in";
  const stageClass = `pg-stage${queen?.facing === "left" ? " pg-stage--flip" : ""}`;
  const cls = [
    "penguin",
    `pg--queen-${pose.replace(/_/g, "-")}`,
    // 벨트는 채워 주기 전까지 **손에** 들려 있다.
    pose === "walk_in" || pose === "belting" ? "pg-belt--held" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={stageClass}>
      <Penguin
        className={cls}
        female
        glam
        onContextMenu={(e) => e.preventDefault()}
      />
    </div>
  );
}
