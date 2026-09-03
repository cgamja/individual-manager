/** 레이어드 SVG 펭귄 — `assets/penguin-icon.png`의 아델리 펭귄을 옮겨 그렸다 (KTD7).
 *
 * **이 파일이 하는 일은 순서를 정하는 것뿐이다.** 도형은 `body`·`hula`·`gear`에,
 * 색은 `../palette`에 있다. **그리는 순서가 곧 겹치는 순서**라 아래 `Shapes()`의
 * 줄 순서를 바꾸면 그림이 바뀐다 — 렌더 스냅샷이 그걸 잡는다.
 */

import { BodyBack, Ground, WingNear } from "./body";
import { Gear } from "./gear";
import { Hula } from "./hula";

interface PenguinOwnProps {
  /** 암컷인가 — 훌라 상의를 입힐지 정한다. 창 라벨에서 결정적으로 파생한다. */
  female?: boolean;
}

type PenguinProps = React.ComponentPropsWithoutRef<"svg"> & PenguinOwnProps;

export function Penguin({ className = "penguin", female = false, ...rest }: PenguinProps) {
  const cls = female ? `${className} pg-female` : className;
  return (
    <svg
      className={cls}
      viewBox="0 0 100 130"
      role="img"
      aria-label="펭귄"
      xmlns="http://www.w3.org/2000/svg"
      {...rest}
    >
      <g className="pg-halo" aria-hidden="true">
        <Shapes />
      </g>
      <Shapes />
    </svg>
  );
}

/** 펭귄을 이루는 도형들. 본체와 후광이 같은 것을 그린다.
 *
 * **한 문서에 두 벌이 들어간다** — `Penguin`이 `<g class="pg-halo">` 안에 한 번,
 * 밖에 한 번 렌더한다. 그래서 **`body`·`hula`·`gear`에 `id`를 쓰면 안 된다**:
 * `<linearGradient id="...">` 같은 것을 넣으면 같은 id가 둘이 되고 `url(#...)`은
 * 먼저 나온 후광 쪽을 집는다. 이 레포가 이미 쓰는 관용구라(`props/court.ts`의
 * 모래 그러데이션) 실수하기 쉽다. 소품 쪽은 창마다 한 벌이라 괜찮다.
 *
 * **`<Hula/>`가 `<BodyBack/>`과 `<WingNear/>` 사이에 있는 것이 핵심이다.**
 * 몸통 위여야 옷으로 읽히고, 날개 아래여야 날개를 저을 때 어깨끈이 안 덮인다. */
function Shapes() {
  return (
    <>
      <Ground />

      <g className="pg-all">
        <BodyBack />
        <Hula />
        <WingNear />
        <Gear />
      </g>
    </>
  );
}
