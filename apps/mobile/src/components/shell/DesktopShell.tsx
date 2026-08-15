import { Grid, VStack } from '@devup-ui/react'
import type { ReactNode } from 'react'

import { AppSidebar } from '@/components/navigation/AppSidebar'
import { BottomTabBar } from '@/components/navigation/BottomTabBar'
import type { AppView } from '@/constants/navigation'

type DesktopShellProps = {
  activeView: AppView
  children: ReactNode
  onNavigate: (view: AppView) => void
}

// 반응형 앱 셸: 넓은 화면은 좌측 사이드바 + 본문, 좁은 화면은 본문 + 하단 탭.
export function DesktopShell({
  activeView,
  children,
  onNavigate,
}: DesktopShellProps) {
  return (
    <Grid
      bg="$background"
      gridTemplateColumns={[
        'minmax(0, 1fr)',
        null,
        '280px minmax(0, 1fr)',
        null,
        '320px minmax(0, 1fr)',
      ]}
      minH="100dvh"
      w="100%"
    >
      <AppSidebar activeView={activeView} onNavigate={onNavigate} />
      <VStack
        as="main"
        gap="28px"
        minW="0"
        pb={['96px', null, '48px']}
        pt={['24px', null, '48px']}
        px={['20px', null, '48px', null, '60px']}
      >
        {children}
      </VStack>
      <BottomTabBar activeView={activeView} onNavigate={onNavigate} />
    </Grid>
  )
}
