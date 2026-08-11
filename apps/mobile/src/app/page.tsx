import { Box, VStack } from "@devup-ui/react";

import { EditorView } from "@/views/EditorView";
import { HistoryView } from "@/views/HistoryView";
import { TranslatorView } from "@/views/TranslatorView";

import { TabBar, TabPanel, TabProvider } from "./TabProvider";

// Page는 앱 프레임을 구성하는 Server Component
// 각 View는 개별 Client Component로 유지
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
  );
}
