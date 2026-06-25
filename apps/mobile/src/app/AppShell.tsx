'use client'

import { Box, Flex } from '@devup-ui/react'
import { useState } from 'react'

import { BottomTabBar, type TabKey } from '@/components/BottomTabBar'
import { EditorView } from '@/views/EditorView'
import { HistoryView } from '@/views/HistoryView'
import { TranslatorView } from '@/views/TranslatorView'

// 탭 상태만 보유하는 최상위 client island.
// 레이아웃/메타데이터는 RSC(layout.tsx, page.tsx)에 남겨둔다.
export function AppShell() {
  const [tab, setTab] = useState<TabKey>('translator')

  return (
    <Flex bg="$bg" flexDirection="column" minHeight="100dvh">
      <Box
        as="main"
        flex={1}
        overflowY="auto"
        pt="calc(env(safe-area-inset-top, 0px) + 8px)"
      >
        {tab === 'translator' && <TranslatorView />}
        {tab === 'editor' && <EditorView />}
        {tab === 'history' && <HistoryView />}
      </Box>
      <BottomTabBar active={tab} onChange={setTab} />
    </Flex>
  )
}
