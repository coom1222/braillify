import { Box, Flex, Text } from "@devup-ui/react";
import { BrailleCellIcon } from "./BrailleCellIcon";

export type TabKey = "translator" | "editor" | "history";

const TABS: Array<{ key: TabKey; label: string; dots: number[] }> = [
  { key: "translator", label: "점역기", dots: [1] },
  { key: "editor", label: "편집기", dots: [1, 4] },
  { key: "history", label: "히스토리", dots: [4, 5] },
];

type Props = {
  active: TabKey;
  onChange: (key: TabKey) => void;
};

export function BottomTabBar({ active, onChange }: Props) {
  return (
    <Box
      as="nav"
      position="sticky"
      bottom={0}
      display="grid"
      gridTemplateColumns="repeat(3, 1fr)"
      bg="$tabBar"
      pt="10px"
      pb="calc(env(safe-area-inset-bottom, 0px) + 8px)"
    >
      {TABS.map((tab) => {
        const isActive = tab.key === active;
        return (
          <Flex
            key={tab.key}
            as="button"
            type="button"
            flexDirection="column"
            alignItems="center"
            gap="4px"
            bg="transparent"
            border={0}
            pt="6px"
            color={isActive ? "#FFFFFF" : "#9A9A9A"}
            onClick={() => onChange(tab.key)}
          >
            <BrailleCellIcon
              dots={tab.dots}
              size={26}
              color={isActive ? "#FFFFFF" : "#9A9A9A"}
            />
            <Text fontSize="12px" fontWeight={isActive ? 600 : 400}>
              {tab.label}
            </Text>
          </Flex>
        );
      })}
    </Box>
  );
}
