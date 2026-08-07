import { Box, VStack } from '@devup-ui/react'

import { EditorView } from '@/views/EditorView'
import { HistoryView } from '@/views/HistoryView'
import { TranslatorView } from '@/views/TranslatorView'

import { TabBar, TabPanel, TabProvider } from './TabProvider'

// RSC: 앱 프레임(전체 높이 컨테이너 · main 영역 · 탭바 배치)을 서버에서 렌더하고,
// client 로 내려가는 것은 탭 상태를 다루는 TabProvider / TabPanel / TabBar 뿐이다.
export default function Page() {
  return (
    <VStack bg="$bg" minHeight="100dvh">
      <TabProvider>
        <Box
          as="main"
          flex={1}
          overflowY="auto"
          pt="calc(env(safe-area-inset-top, 0px) + 8px)"
        >
          <TabPanel tab="translator">
            <TranslatorView />
          </TabPanel>
          <TabPanel tab="editor">
            <EditorView />
          </TabPanel>
          <TabPanel tab="history">
            <HistoryView />
          </TabPanel>
        </Box>
        <TabBar />
      </TabProvider>
    </VStack>
  )
}
