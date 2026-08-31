import { Box, Flex, Text, VStack } from '@devup-ui/react'

/** Displays the latest full-corpus regression result on the landing page. */
export function CorpusAccuracy() {
  return (
    <Box
      aria-labelledby="corpus-accuracy-title"
      as="section"
      bg="$containerBackground"
      border="1px solid $text"
      borderRadius={['20px', null, null, '32px']}
      maxW="1520px"
      mx="auto"
      overflow="hidden"
      px={['24px', null, null, '60px']}
      py={['28px', null, null, '48px']}
      w="100%"
    >
      <Flex
        alignItems={['flex-start', null, null, 'center']}
        flexDir={['column', null, null, 'row']}
        gap={['28px', null, null, '60px']}
        justifyContent="space-between"
      >
        <VStack gap="12px" maxW="620px">
          <Text color="$caption" typography="bodyBold">
            NIKL Korean–Korean Braille Parallel Corpus 2025 v1.0
          </Text>
          <Text
            as="h2"
            color="$text"
            id="corpus-accuracy-title"
            m="0"
            typography="featureTitle"
          >
            국립국어원 말뭉치 전수 검증 현황
          </Text>
          <Text color="$text" typography="body" wordBreak="keep-all">
            83,528개 문장을 점역해 기준 점자와 문장 단위로 완전 비교합니다.
            점자 공백 표기는 동일한 형태로 정규화합니다.
          </Text>
        </VStack>
        <Flex
          flexDir={['column', null, 'row']}
          gap={['16px', null, '24px']}
          w={['100%', null, 'auto']}
        >
          <VStack
            bg="$background"
            borderRadius="16px"
            gap="4px"
            minW={['100%', null, '150px']}
            p="20px"
          >
            <Text color="$caption" typography="bodyBold">
              문장 단위 완전 일치율
            </Text>
            <Text color="$text" typography="title">
              69.1%
            </Text>
          </VStack>
          <VStack
            bg="$background"
            borderRadius="16px"
            gap="4px"
            minW={['100%', null, '150px']}
            p="20px"
          >
            <Text color="$caption" typography="bodyBold">
              일치 문장
            </Text>
            <Text color="$text" typography="title">
              57,732 / 83,528
            </Text>
          </VStack>
        </Flex>
      </Flex>
    </Box>
  )
}
