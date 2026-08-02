import { VStack } from '@devup-ui/react'

import { AppShell } from './AppShell'

// RSC: 정적 레이아웃 프레임(전체 높이 컨테이너)을 담당하고,
// 상호작용(탭 상태)은 AppShell client island 로 위임한다.
export default function Page() {
  return (
    <VStack bg="$bg" minHeight="100dvh">
      <AppShell />
    </VStack>
  )
}
