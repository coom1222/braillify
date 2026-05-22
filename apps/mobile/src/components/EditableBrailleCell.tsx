import { Box, Flex, Text } from "@devup-ui/react";
import { type DotNumber } from "../lib/braille";

type Props = {
  mask: number;
  index: number;
  onToggleDot: (dot: DotNumber) => void;
  onRemove?: () => void;
};

const DOT_LAYOUT: Array<{ dot: DotNumber }> = [
  { dot: 1 },
  { dot: 2 },
  { dot: 3 },
  { dot: 4 },
  { dot: 5 },
  { dot: 6 },
];

function dotBit(dot: DotNumber): number {
  return 1 << (dot - 1);
}

export function EditableBrailleCell({
  mask,
  index,
  onToggleDot,
  onRemove,
}: Props) {
  return (
    <Flex flexDirection="column" alignItems="center" gap="6px">
      <Text fontSize="11px" color="$textSubtle">
        #{index + 1}
      </Text>
      <Box bg="$bg" borderRadius="12px" px="14px" py="12px">
        <Box
          display="grid"
          gridTemplateColumns="repeat(2, 22px)"
          gridTemplateRows="repeat(3, 22px)"
          gridAutoFlow="column"
          gap="6px"
        >
          {DOT_LAYOUT.map(({ dot }) => {
            const active = (mask & dotBit(dot)) !== 0;
            return (
              <Box
                key={dot}
                as="button"
                type="button"
                w="22px"
                h="22px"
                borderRadius="50%"
                border="1.5px solid"
                borderColor={active ? "$primary" : "$border"}
                bg={active ? "$primary" : "$surface"}
                p={0}
                aria-label={`${index + 1}번 셀 점 ${dot}`}
                aria-pressed={active}
                onClick={() => onToggleDot(dot)}
              />
            );
          })}
        </Box>
      </Box>
      {onRemove && (
        <Box
          as="button"
          type="button"
          bg="transparent"
          border={0}
          color="$textSubtle"
          fontSize="18px"
          lineHeight={1}
          p={0}
          aria-label="셀 삭제"
          onClick={onRemove}
        >
          ×
        </Box>
      )}
    </Flex>
  );
}
