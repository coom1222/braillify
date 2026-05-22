import { useState } from "react";
import { Flex, Box } from "@devup-ui/react";
import { BottomTabBar, type TabKey } from "./components/BottomTabBar";
import { TranslatorPage } from "./pages/TranslatorPage";
import { EditorPage } from "./pages/EditorPage";
import { HistoryPage } from "./pages/HistoryPage";

function App() {
  const [tab, setTab] = useState<TabKey>("translator");

  return (
    <Flex flexDirection="column" minHeight="100dvh" bg="$bg">
      <Box
        flex={1}
        overflowY="auto"
        as="main"
        pt="calc(env(safe-area-inset-top, 0px) + 8px)"
      >
        {tab === "translator" && <TranslatorPage />}
        {tab === "editor" && <EditorPage />}
        {tab === "history" && <HistoryPage />}
      </Box>
      <BottomTabBar active={tab} onChange={setTab} />
    </Flex>
  );
}

export default App;
