'use client'

import { Button, Flex, Text, VStack } from '@devup-ui/react'
import { useMemo, useState } from 'react'

import { useAppState } from '@/components/shell/AppShell'
import { MODE_CONTENT } from '@/constants/translation'
import { copyText } from '@/lib/clipboard'
import type { HistoryEntry } from '@/lib/history'

function formatCreatedAt(value: string): string {
  return new Intl.DateTimeFormat('ko-KR', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value))
}

export function HistoryPanel() {
  const { deleteAll, deleteEntry, entries, requestRestore, toggleFavorite } =
    useAppState()
  const [favoritesOnly, setFavoritesOnly] = useState(false)
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [copyErrorId, setCopyErrorId] = useState<string | null>(null)
  const visibleEntries = useMemo(
    () => (favoritesOnly ? entries.filter((entry) => entry.favorite) : entries),
    [entries, favoritesOnly],
  )

  const copyEntry = async (entry: HistoryEntry) => {
    try {
      await copyText(entry.result)
      setCopiedId(entry.id)
      setCopyErrorId(null)
    } catch {
      setCopiedId(null)
      setCopyErrorId(entry.id)
    }
  }

  return (
    <VStack gap="20px" w="100%">
      <Flex
        alignItems="center"
        bg="$containerBackground"
        border="1px solid $border"
        borderRadius="16px"
        flexWrap="wrap"
        gap="12px"
        justifyContent="space-between"
        px="20px"
        py="16px"
      >
        <Flex aria-label="히스토리 필터" as="fieldset" border="none" gap="8px">
          <Button
            aria-pressed={!favoritesOnly}
            bg={!favoritesOnly ? '$primary' : '$background'}
            border="1px solid $border"
            borderRadius="10px"
            color={!favoritesOnly ? '$base' : '$text'}
            cursor="pointer"
            onClick={() => setFavoritesOnly(false)}
            px="16px"
            py="10px"
            type="button"
          >
            전체 {entries.length}
          </Button>
          <Button
            aria-pressed={favoritesOnly}
            bg={favoritesOnly ? '$primary' : '$background'}
            border="1px solid $border"
            borderRadius="10px"
            color={favoritesOnly ? '$base' : '$text'}
            cursor="pointer"
            onClick={() => setFavoritesOnly(true)}
            px="16px"
            py="10px"
            type="button"
          >
            즐겨찾기 {entries.filter((entry) => entry.favorite).length}
          </Button>
        </Flex>
        <Button
          bg="transparent"
          border="1px solid $border"
          borderRadius="10px"
          color="$error"
          cursor={entries.length ? 'pointer' : 'default'}
          disabled={!entries.length}
          onClick={deleteAll}
          px="14px"
          py="9px"
          type="button"
        >
          전체 삭제
        </Button>
      </Flex>

      {visibleEntries.length === 0 ? (
        <VStack
          alignItems="center"
          bg="$containerBackground"
          border="1px solid $border"
          borderRadius="20px"
          gap="14px"
          justifyContent="center"
          minH="360px"
          p="36px"
        >
          <Text aria-hidden="true" color="$emptyBraille" fontSize="42px">
            ⠶⠶⠶
          </Text>
          <Text color="$caption" typography="body">
            {favoritesOnly
              ? '즐겨찾기한 점역 기록이 없습니다.'
              : '아직 저장된 점역 기록이 없습니다.'}
          </Text>
        </VStack>
      ) : (
        <VStack gap="14px">
          {visibleEntries.map((entry) => (
            <VStack
              key={entry.id}
              as="article"
              bg="$containerBackground"
              border="1px solid $border"
              borderRadius="18px"
              gap="14px"
              p="20px"
            >
              <Flex
                alignItems="center"
                gap="10px"
                justifyContent="space-between"
              >
                <Flex alignItems="center" flexWrap="wrap" gap="10px">
                  <Text
                    bg="$background"
                    borderRadius="999px"
                    color="$caption"
                    px="10px"
                    py="5px"
                    typography="sidebarCaption"
                  >
                    {entry.mode === 'math'
                      ? '수학'
                      : entry.mode === 'reverse'
                        ? '역점역'
                        : '일반'}
                  </Text>
                  <Text color="$caption" typography="sidebarCaption">
                    {formatCreatedAt(entry.createdAt)}
                  </Text>
                </Flex>
                <Button
                  aria-label={`${entry.favorite ? '즐겨찾기 해제' : '즐겨찾기 추가'}: ${entry.input}`}
                  bg="transparent"
                  border="none"
                  color={entry.favorite ? '$focus' : '$caption'}
                  cursor="pointer"
                  fontSize="24px"
                  onClick={() => toggleFavorite(entry.id)}
                  px="8px"
                  py="4px"
                  type="button"
                >
                  {entry.favorite ? '★' : '☆'}
                </Button>
              </Flex>

              <VStack gap="5px">
                <Text color="$caption" typography="sidebarCaption">
                  {MODE_CONTENT[entry.mode].inputLabel}
                </Text>
                <Text color="$text" typography="body" whiteSpace="pre-wrap">
                  {entry.input}
                </Text>
              </VStack>
              <Text
                aria-label="저장된 점역 결과"
                bg="$background"
                borderRadius="12px"
                color="$text"
                fontSize={entry.mode === 'reverse' ? '18px' : '25px'}
                p="14px"
                typography={
                  entry.mode === 'reverse' ? 'reverseText' : 'braille'
                }
                whiteSpace="pre-wrap"
                wordBreak="break-word"
              >
                {entry.result}
              </Text>

              <Flex
                alignItems="center"
                flexWrap="wrap"
                gap="8px"
                justifyContent="flex-end"
              >
                <Text
                  color={copyErrorId === entry.id ? '$error' : '$caption'}
                  mr="auto"
                  role={copyErrorId === entry.id ? 'alert' : 'status'}
                  typography="sidebarCaption"
                >
                  {copiedId === entry.id
                    ? '복사했습니다.'
                    : copyErrorId === entry.id
                      ? '복사하지 못했습니다.'
                      : ''}
                </Text>
                <Button
                  bg="$background"
                  border="1px solid $border"
                  borderRadius="9px"
                  color="$text"
                  cursor="pointer"
                  onClick={() => copyEntry(entry)}
                  px="12px"
                  py="8px"
                  type="button"
                >
                  결과 복사
                </Button>
                <Button
                  bg="$primary"
                  border="none"
                  borderRadius="9px"
                  color="$base"
                  cursor="pointer"
                  onClick={() => requestRestore(entry)}
                  px="12px"
                  py="9px"
                  type="button"
                >
                  점역기로 불러오기
                </Button>
                <Button
                  aria-label={`기록 삭제: ${entry.input}`}
                  bg="transparent"
                  border="1px solid $border"
                  borderRadius="9px"
                  color="$error"
                  cursor="pointer"
                  onClick={() => deleteEntry(entry.id)}
                  px="12px"
                  py="8px"
                  type="button"
                >
                  삭제
                </Button>
              </Flex>
            </VStack>
          ))}
        </VStack>
      )}
    </VStack>
  )
}
