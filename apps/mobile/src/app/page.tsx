import { BrailleEditor } from '@/components/editor/BrailleEditor'
import { HistoryPanel } from '@/components/history/HistoryPanel'
import { AppShell } from '@/components/shell/AppShell'
import { ViewPanel } from '@/components/shell/ViewPanel'
import { TranslatorWorkspace } from '@/components/translator/TranslatorWorkspace'

// 서버 컴포넌트: 앱 프레임과 화면들을 조합만 한다. 상태가 필요한 부분은
// AppShell(client island)과 각 화면 내부의 작은 client 컴포넌트가 담당한다.
export default function HomePage() {
  return (
    <AppShell>
      <ViewPanel
        description="일반 텍스트와 LaTeX 수식을 점역하거나 한국 점자를 한글로 역점역합니다."
        title="점역기"
        view="translator"
      >
        <TranslatorWorkspace />
      </ViewPanel>
      <ViewPanel
        description="6점 조합으로 점자 셀을 만들거나 유니코드 점자를 직접 편집합니다."
        title="점자 편집기"
        view="editor"
      >
        <BrailleEditor />
      </ViewPanel>
      <ViewPanel
        description="최근 점역 결과를 확인하고 즐겨찾기, 복사, 삭제 또는 다시 불러오기를 할 수 있습니다."
        title="히스토리"
        view="history"
      >
        <HistoryPanel />
      </ViewPanel>
    </AppShell>
  )
}
