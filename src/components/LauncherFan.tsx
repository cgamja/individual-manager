import { useCallback, useEffect, useState, type ReactNode } from "react";
import { ServiceIcon, serviceColor } from "./ServiceIcon";
import { fanOffset, isOpenableUrl, type LauncherService } from "../lib/launcher";

interface LauncherFanProps {
  services: readonly LauncherService[];
  /** 서비스 URL 열기. 실패하면 reject해야 그 자리에 알릴 수 있다. */
  onOpen: (url: string) => Promise<void>;
  /** 뽀모도로 카드의 2단 — URL 목록이 아니라 타이머가 펼쳐진다 (R4). */
  timerPanel: ReactNode;
  /** 접을 게 없는데 Esc가 왔다 — 바깥이 다음 단계(설정 접기 / 팝오버 닫기)를 정한다 (R6). */
  onEscape?: () => void;
}

/** 항목 하나가 열리지 않았을 때 그 자리에 대신 보여줄 문구. */
type ItemNote = { serviceId: string; label: string; text: string };

/**
 * 런처 — 카드 넉 장이 호를 그리며 펼쳐지고, 누르면 2단이 열린다.
 *
 * 보이는 건 부채지만 DOM은 평범한 세로 버튼 목록이다 (KTD2). 호는 CSS transform이
 * 만들고 JS 애니메이션 루프는 두지 않는다 — 숨겨진 웹뷰의 JS 타이머는 ~5분 뒤 멈추고,
 * 팝오버는 숨겨졌다 다시 보이기를 반복한다. 목록 구조라 Tab 순서와 스크린리더도 따라온다.
 */
export function LauncherFan({ services, onOpen, timerPanel, onEscape }: LauncherFanProps) {
  /** 펼쳐진 카드 id 하나. 이게 2단 상태의 전부다 — 한 번에 하나만 열린다 (R2). */
  const [openId, setOpenId] = useState<string | null>(null);
  const [note, setNote] = useState<ItemNote | null>(null);

  const toggle = useCallback((id: string) => {
    setNote(null);
    setOpenId((current) => (current === id ? null : id));
  }, []);

  // Esc는 펼쳐진 걸 먼저 접고, 접을 게 없으면 바깥에 넘긴다 (R6)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (openId === null) {
        onEscape?.();
        return;
      }
      setOpenId(null);
      setNote(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [openId, onEscape]);

  const handleItem = useCallback(
    async (service: LauncherService, label: string, url: string) => {
      // 아직 채우지 않은 URL이다 — 열기를 시도하지 않고 그 자리에 알린다 (R7)
      if (!isOpenableUrl(url)) {
        setNote({ serviceId: service.id, label, text: "URL이 아직 없어요" });
        return;
      }
      setNote(null);
      try {
        await onOpen(url);
      } catch {
        // 런처는 닫지 않는다 — 사용자가 다른 항목을 바로 다시 고를 수 있게
        setNote({ serviceId: service.id, label, text: "열지 못했어요" });
      }
    },
    [onOpen],
  );

  return (
    <ul className="fan">
      {services.map((service, index) => {
        const { dx, rotate } = fanOffset(index, services.length);
        const expanded = openId === service.id;
        return (
          <li
            key={service.id}
            className="fan-row"
            style={
              {
                "--fan-dx": `${dx}px`,
                "--fan-rotate": `${rotate}deg`,
                "--fan-color": serviceColor(service.id),
              } as React.CSSProperties
            }
          >
            <button
              type="button"
              className="fan-card"
              aria-expanded={expanded}
              onClick={() => toggle(service.id)}
            >
              <span className="fan-pill">{service.label}</span>
              <span className="fan-tile">
                <ServiceIcon id={service.id} />
              </span>
            </button>
            {expanded &&
              (service.items.length === 0 ? (
                <div className="fan-sub">{timerPanel}</div>
              ) : (
                <ul className="fan-sub">
                  {service.items.map((item) => {
                    const shown =
                      note && note.serviceId === service.id && note.label === item.label
                        ? note.text
                        : null;
                    return (
                      <li key={item.label}>
                        <button
                          type="button"
                          className={`fan-item${shown ? " fan-item--blocked" : ""}`}
                          onClick={() => void handleItem(service, item.label, item.url)}
                        >
                          <span>{item.label}</span>
                          <span className="fan-item-note">{shown ?? "Chrome ↗"}</span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              ))}
          </li>
        );
      })}
    </ul>
  );
}
