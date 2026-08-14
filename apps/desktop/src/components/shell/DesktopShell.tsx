import { Grid, VStack } from '@devup-ui/react'
import type { ReactNode } from 'react'

import { AppSidebar } from '@/components/navigation/AppSidebar'
import type { AppView } from '@/constants/navigation'

type DesktopShellProps = {
  activeView: AppView
  children: ReactNode
  onNavigate: (view: AppView) => void
}

export function DesktopShell({
  activeView,
  children,
  onNavigate,
}: DesktopShellProps) {
  return (
    <Grid
      bg="$background"
      gridTemplateColumns={[
        '260px minmax(0, 1fr)',
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
        px={['36px', null, '48px', null, '60px']}
        py={['36px', null, '48px']}
      >
        {children}
      </VStack>
    </Grid>
  )
}
