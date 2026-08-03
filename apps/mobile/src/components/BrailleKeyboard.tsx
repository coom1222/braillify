import { Box, Flex, Grid, Text } from '@devup-ui/react'

import { type DotNumber } from '../lib/braille'
import { EditableBrailleCell } from './EditableBrailleCell'

type BrailleKeyboardProps = {
  cells: number[]
  cellIds: number[]
  onToggleDot: (cellIndex: number, dot: DotNumber) => void
  onRemoveCell: (cellIndex: number) => void
  onBackspace: () => void
  onAddSpace: () => void
  onAddCell: () => void
  onClear: () => void
}

export function BrailleKeyboard({
  cells,
  cellIds,
  onToggleDot,
  onRemoveCell,
  onBackspace,
  onAddSpace,
  onAddCell,
  onClear,
}: BrailleKeyboardProps) {
  const hasContent = cells.length > 0

  return (
    <Box>
      <Text color="$textSubtle" fontSize="12px" fontWeight={600} mb="8px">
        도트 키보드
      </Text>

      {hasContent ? (
        <Grid
          gap="12px"
          gridTemplateColumns="repeat(auto-fill, minmax(80px, 1fr))"
          maxH="220px"
          mb="12px"
          overflowY="auto"
        >
          {cells.map((mask, index) => (
            <EditableBrailleCell
              key={cellIds[index] ?? index}
              index={index}
              mask={mask}
              onRemove={() => onRemoveCell(index)}
              onToggleDot={(dot) => onToggleDot(index, dot)}
            />
          ))}
        </Grid>
      ) : (
        <Flex
          alignItems="center"
          border="1px dashed"
          borderColor="$border"
          borderRadius="8px"
          color="$textSubtle"
          fontSize="12px"
          justifyContent="center"
          mb="12px"
          py="16px"
        >
          왼쪽: 점 1·2·3 &nbsp;/&nbsp; 오른쪽: 점 4·5·6
        </Flex>
      )}

      <Flex flexWrap="wrap" gap="8px">
        <Box
          aria-label="마지막 셀 지우기"
          as="button"
          bg="$surface"
          border="1px solid"
          borderColor="$border"
          borderRadius="8px"
          color="$text"
          cursor={!hasContent ? 'not-allowed' : 'pointer'}
          disabled={!hasContent}
          fontSize="14px"
          fontWeight={500}
          onClick={onBackspace}
          opacity={!hasContent ? 0.4 : 1}
          px="14px"
          py="9px"
          type="button"
        >
          ⌫
        </Box>

        <Box
          as="button"
          bg="$surface"
          border="1px solid"
          borderColor="$border"
          borderRadius="8px"
          color="$text"
          cursor="pointer"
          fontSize="13px"
          fontWeight={500}
          onClick={onAddSpace}
          px="14px"
          py="9px"
          type="button"
        >
          공백 ⠀
        </Box>

        <Box
          as="button"
          bg="$primary"
          border={0}
          borderRadius="8px"
          color="$primaryText"
          cursor="pointer"
          fontSize="13px"
          fontWeight={600}
          onClick={onAddCell}
          px="16px"
          py="9px"
          type="button"
        >
          + 셀
        </Box>

        {hasContent && (
          <Box
            as="button"
            bg="transparent"
            border="1px solid"
            borderColor="$danger"
            borderRadius="8px"
            color="$danger"
            cursor="pointer"
            fontSize="13px"
            fontWeight={500}
            onClick={onClear}
            px="14px"
            py="9px"
            type="button"
          >
            전체 삭제
          </Box>
        )}
      </Flex>
    </Box>
  )
}
