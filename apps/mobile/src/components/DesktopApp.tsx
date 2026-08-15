'use client'

import { Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

import { BrailleEditor } from '@/components/editor/BrailleEditor'
import { HistoryPanel } from '@/components/history/HistoryPanel'
import { DesktopShell } from '@/components/shell/DesktopShell'
import {
  type TranslationDraft,
  TranslatorWorkspace,
} from '@/components/translator/TranslatorWorkspace'
import type { AppView } from '@/constants/navigation'
import { useTranslationHistory } from '@/hooks/useTranslationHistory'
import type { HistoryEntry } from '@/lib/history'

const PAGE_CONTENT = {
  editor: {
    description:
      '6점 조합으로 점자 셀을 만들거나 유니코드 점자를 직접 편집합니다.',
    title: '점자 편집기',
  },
  history: {
    description:
      '최근 점역 결과를 확인하고 즐겨찾기, 복사, 삭제 또는 다시 불러오기를 할 수 있습니다.',
    title: '히스토리',
  },
  translator: {
    description:
      '일반 텍스트와 LaTeX 수식을 점역하거나 한국 점자를 한글로 역점역합니다.',
    title: '점역기',
  },
} as const satisfies Record<AppView, { description: string; title: string }>

export function DesktopApp() {
  const [activeView, setActiveView] = useState<AppView>('translator')
  const [translationDraft, setTranslationDraft] =
    useState<TranslationDraft | null>(null)
  const [restoreRequestId, setRestoreRequestId] = useState(0)
  const { addEntry, deleteAll, deleteEntry, entries, toggleFavorite } =
    useTranslationHistory()
  const pageContent = PAGE_CONTENT[activeView]

  const restoreHistoryEntry = (entry: HistoryEntry) => {
    const nextRequestId = restoreRequestId + 1
    setRestoreRequestId(nextRequestId)
    setTranslationDraft({
      input: entry.input,
      mode: entry.mode,
      requestId: nextRequestId,
      result: entry.result,
    })
    setActiveView('translator')
  }

  return (
    <DesktopShell activeView={activeView} onNavigate={setActiveView}>
      <VStack gap="8px">
        <Text as="h2" typography="pageTitle">
          {pageContent.title}
        </Text>
        <Text color="$caption" typography="pageDescription">
          {pageContent.description}
        </Text>
      </VStack>

      <div hidden={activeView !== 'translator'}>
        <TranslatorWorkspace
          initialDraft={translationDraft}
          onAddHistory={addEntry}
        />
      </div>
      <div hidden={activeView !== 'editor'}>
        <BrailleEditor />
      </div>
      <div hidden={activeView !== 'history'}>
        <HistoryPanel
          entries={entries}
          onClear={deleteAll}
          onDelete={deleteEntry}
          onRestore={restoreHistoryEntry}
          onToggleFavorite={toggleFavorite}
        />
      </div>
    </DesktopShell>
  )
}
