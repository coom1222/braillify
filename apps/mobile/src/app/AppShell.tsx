'use client'

import { Box, VStack } from '@devup-ui/react'
import { useEffect, useState } from 'react'

import { BottomTabBar, type TabKey } from '@/components/BottomTabBar'
import { EditorView } from '@/views/EditorView'
import { HistoryView } from '@/views/HistoryView'
import { TranslatorView } from '@/views/TranslatorView'

const TAB_KEYS: readonly TabKey[] = ['translator', 'editor', 'history']

// URL 쿼리스트링(?tab=)에서 유효한 탭만 읽고, 없으면 기본 탭으로
function readTabFromUrl(): TabKey {
  const tab = new URLSearchParams(window.location.search).get('tab')
  return TAB_KEYS.find((k) => k === tab) ?? 'translator'
}

// 탭 상태만 보유하는 최상위 client island.
// 레이아웃/메타데이터는 RSC(layout.tsx, page.tsx)에 남겨둔다.
export function AppShell() {
  const [tab, setTab] = useState<TabKey>('translator')

  // mount 후 URL → 탭 동기화 (정적 export prerender 를 깨지 않도록 effect 에서)
  useEffect(() => {
    setTab(readTabFromUrl())
  }, [])

  // 탭 전환 시 상태와 URL(?tab=) 을 함께 갱신 (히스토리 스택은 쌓지 않음)
  function changeTab(next: TabKey) {
    setTab(next)
    const url = new URL(window.location.href)
    url.searchParams.set('tab', next)
    window.history.replaceState(null, '', url)
  }

  return (
    <VStack bg="$bg" minHeight="100dvh">
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
      <BottomTabBar active={tab} onChange={changeTab} />
    </VStack>
  )
}
