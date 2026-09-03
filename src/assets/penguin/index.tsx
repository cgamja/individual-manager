/** 레이어드 SVG 펭귄 — `assets/penguin-icon.png`의 아델리 펭귄을 옮겨 그렸다.
 *
 * 도형은 `body`·`hula`·`gear`에, 색은 `../palette`에 있다. 이 파일은 순서만 정한다.
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
 * **그리는 순서 = 겹치는 순서.** `<Hula/>`가 `<BodyBack/>`과 `<WingNear/>`
 * 사이여야 옷이 몸통 위·날개 아래에 온다.
 *
 * **부위 파일에 `id`를 쓰면 안 된다** — 한 문서에 두 벌이 들어가서
 * `<linearGradient id>` 같은 것은 id가 겹치고 `url(#…)`이 후광 쪽을 집는다. */
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
