'use client'

import { createContext, useContext, useEffect, useState } from 'react'

import { BottomTabBar, type TabKey } from '@/components/BottomTabBar'

const TAB_KEYS = [
  'translator',
  'editor',
  'history',
] as const satisfies readonly TabKey[]

const DEFAULT_TAB: TabKey = 'translator'

// URL 쿼리스트링(?tab=)에서 유효한 탭만 읽고, 없으면 기본 탭으로
function readTabFromUrl(): TabKey {
  const tab = new URLSearchParams(window.location.search).get('tab')
  return TAB_KEYS.find((k) => k === tab) ?? DEFAULT_TAB
}

interface TabState {
  tab: TabKey
  changeTab: (next: TabKey) => void
}

const TabContext = createContext<TabState | null>(null)

function useTabState(): TabState {
  const state = useContext(TabContext)
  if (!state)
    throw new Error('TabProvider 바깥에서는 탭 상태를 쓸 수 없습니다.')
  return state
}

// 탭 상태만 들고 있는 client island.
// children 은 RSC 에서 그대로 넘어오므로 프레임 마크업은 서버에 남는다.
export function TabProvider({ children }: { children: React.ReactNode }) {
  const [tab, setTab] = useState<TabKey>(DEFAULT_TAB)

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

  return <TabContext value={{ tab, changeTab }}>{children}</TabContext>
}

// 활성 탭일 때만 children 을 렌더한다. children 자체는 서버에서 내려온다.
export function TabPanel({
  children,
  tab,
}: {
  children: React.ReactNode
  tab: TabKey
}) {
  return useTabState().tab === tab ? <>{children}</> : null
}

// 프레젠테이션 컴포넌트인 BottomTabBar 를 탭 상태에 연결한다.
export function TabBar() {
  const { tab, changeTab } = useTabState()
  return <BottomTabBar active={tab} onChange={changeTab} />
}
