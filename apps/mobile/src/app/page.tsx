import { AppShell } from './AppShell'

// RSC: 정적 셸만 담당하고, 상호작용(탭 상태)은 AppShell client island 로 위임한다.
export default function Page() {
  return <AppShell />
}
