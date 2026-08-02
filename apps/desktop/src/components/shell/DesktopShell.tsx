import { Grid, VStack } from '@devup-ui/react'
import type { ReactNode } from 'react'

import { AppSidebar } from '@/components/navigation/AppSidebar'

type DesktopShellProps = {
  children: ReactNode
}

export function DesktopShell({ children }: DesktopShellProps) {
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
      <AppSidebar />
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
