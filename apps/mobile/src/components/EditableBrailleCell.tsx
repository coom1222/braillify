import { Box, Grid, Text, VStack } from '@devup-ui/react'

import { type DotNumber } from '../lib/braille'

type Props = {
  mask: number
  index: number
  onToggleDot: (dot: DotNumber) => void
  onRemove?: () => void
}

const DOT_LAYOUT = [1, 2, 3, 4, 5, 6] as const

function dotBit(dot: DotNumber): number {
  return 1 << (dot - 1)
}

export function EditableBrailleCell({
  mask,
  index,
  onToggleDot,
  onRemove,
}: Props) {
  return (
    <VStack alignItems="center" gap="6px">
      <Text color="$textSubtle" fontSize="11px">
        #{index + 1}
      </Text>
      <Box bg="$bg" borderRadius="12px" px="14px" py="12px">
        <Grid
          gap="6px"
          gridAutoFlow="column"
          gridTemplateColumns="repeat(2, 22px)"
          gridTemplateRows="repeat(3, 22px)"
        >
          {DOT_LAYOUT.map((dot) => {
            const active = (mask & dotBit(dot)) !== 0
            return (
              <Box
                key={dot}
                aria-label={`${index + 1}번 셀 점 ${dot}`}
                aria-pressed={active}
                as="button"
                bg={active ? '$primary' : '$surface'}
                border="1.5px solid"
                borderColor={active ? '$primary' : '$border'}
                borderRadius="50%"
                h="22px"
                onClick={() => onToggleDot(dot)}
                p={0}
                type="button"
                w="22px"
              />
            )
          })}
        </Grid>
      </Box>
      {onRemove && (
        <Box
          aria-label="셀 삭제"
          as="button"
          bg="transparent"
          border={0}
          color="$textSubtle"
          fontSize="18px"
          lineHeight={1}
          onClick={onRemove}
          p={0}
          type="button"
        >
          ×
        </Box>
      )}
    </VStack>
  )
}
