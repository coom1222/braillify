import { Text, VStack } from '@devup-ui/react'

import { DesktopShell } from '@/components/shell/DesktopShell'
import { TranslatorWorkspace } from '@/components/translator/TranslatorWorkspace'

export default function HomePage() {
  return (
    <DesktopShell>
      <VStack gap="8px">
        <Text as="h2" typography="pageTitle">
          점역기
        </Text>
        <Text color="$caption" typography="pageDescription">
          한글 텍스트를 입력하면 2024 개정 한국 점자 규정에 따라 점역합니다.
        </Text>
      </VStack>
      <TranslatorWorkspace />
    </DesktopShell>
  )
}
