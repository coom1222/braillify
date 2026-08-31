import { Box, Flex, Text, VStack } from '@devup-ui/react'

interface CorpusSample {
  input: string
  unicode: string
  world: string
}

interface CorpusTestCaseSectionProps {
  total: number
  braillifyMatch: number
  worldMatch: number
  samples: CorpusSample[]
}

/** NIKL 병렬 말뭉치의 빌드 시점 검증 현황과 표본을 표시한다. */
export function CorpusTestCaseSection({
  total,
  braillifyMatch,
  worldMatch,
  samples,
}: CorpusTestCaseSectionProps) {
  const braillifyAccuracy = ((braillifyMatch / total) * 100).toFixed(2)
  const worldAccuracy = ((worldMatch / total) * 100).toFixed(2)

  return (
    <VStack alignItems="stretch" gap="20px">
      <VStack alignItems="stretch" gap="8px">
        <Text color="$title" typography="docsTitle">
          NIKL 한국어–한국어 점자 병렬 말뭉치 2025 v1.0
        </Text>
        <Text color="$text" typography="body" wordBreak="keep-all">
          국립국어원 말뭉치 {total.toLocaleString()}문장을 빌드 시점에 다시
          읽어 점역 결과를 집계합니다. 아래에는 원문·기준 점형·World 결과의
          표본을 표시합니다.
        </Text>
      </VStack>

      <Flex flexWrap="wrap" gap="12px">
        <Box bg="$menuHover" borderRadius="10px" px="16px" py="12px">
          <Text color="$caption" typography="docsCaption">
            Braillify 문장 완전 일치
          </Text>
          <Text color="$title" typography="featureTitle">
            {braillifyAccuracy}% ({braillifyMatch.toLocaleString()} /{' '}
            {total.toLocaleString()})
          </Text>
        </Box>
        <Box bg="$menuHover" borderRadius="10px" px="16px" py="12px">
          <Text color="$caption" typography="docsCaption">
            World 문장 완전 일치
          </Text>
          <Text color="$title" typography="featureTitle">
            {worldAccuracy}% ({worldMatch.toLocaleString()} /{' '}
            {total.toLocaleString()})
          </Text>
        </Box>
      </Flex>

      <VStack alignItems="stretch" gap="8px">
        {samples.map((sample, index) => (
          <Box
            border="solid 1px $primary"
            borderRadius="10px"
            key={`${sample.input}-${index}`}
            p="16px"
          >
            <VStack alignItems="stretch" gap="8px">
              <Text color="$caption" typography="docsCaption">
                표본 {index + 1}
              </Text>
              <Text color="$text" typography="body" wordBreak="keep-all">
                {sample.input}
              </Text>
              <Text color="$caption" typography="docsCaption">
                NIKL 기준 점형
              </Text>
              <Text color="$text" overflowWrap="anywhere" typography="body">
                {sample.unicode}
              </Text>
              <Text color="$caption" typography="docsCaption">
                World 결과
              </Text>
              <Text color="$text" overflowWrap="anywhere" typography="body">
                {sample.world}
              </Text>
            </VStack>
          </Box>
        ))}
      </VStack>
    </VStack>
  )
}
