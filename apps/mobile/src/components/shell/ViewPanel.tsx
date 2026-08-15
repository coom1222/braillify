'use client'

import { Text, VStack } from '@devup-ui/react'
import type { ReactNode } from 'react'

import { useAppState } from '@/components/shell/AppShell'
import type { AppView } from '@/constants/navigation'

type ViewPanelProps = {
  children: ReactNode
  description: string
  title: string
  view: AppView
}

// 활성 화면일 때만 노출되는 패널. 화면 본문(children)은 서버에서 조합되어
// 내려오고, 이 컴포넌트는 activeView 에 따라 표시 여부만 제어한다.
export function ViewPanel({
  children,
  description,
  title,
  view,
}: ViewPanelProps) {
  const { activeView } = useAppState()

  return (
    <VStack gap="28px" hidden={activeView !== view}>
      <VStack gap="8px">
        <Text as="h2" typography="pageTitle">
          {title}
        </Text>
        <Text color="$caption" typography="pageDescription">
          {description}
        </Text>
      </VStack>
      {children}
    </VStack>
  )
}
