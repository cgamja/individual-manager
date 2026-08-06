import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

/**
 * 알림 권한을 확인하고 필요하면 요청한다 (최초 실행 시 1회 프롬프트).
 * 거부되거나 오류가 나도 앱은 계속 동작한다 — 이때 UI가 대체 표시를 담당한다 (R8).
 */
export async function ensureNotificationPermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) {
      return true;
    }
    return (await requestPermission()) === "granted";
  } catch {
    return false;
  }
}
