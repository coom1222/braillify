import { Box, Flex, Text } from "@devup-ui/react";

export type TabKey = "translator" | "editor" | "history";

const TABS: Array<{ key: TabKey; label: string; icon: string }> = [
  { key: "translator", label: "점역기", icon: "⠿" },
  { key: "editor", label: "편집기", icon: "⠶" },
  { key: "history", label: "히스토리", icon: "⠒" },
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
      {TABS.map(tab => {
        const isActive = tab.key === active;
        return (
          <Flex
            key={tab.key}
            as="button"
            type="button"
            flexDirection="column"
            alignItems="center"
            justifyContent="center"
            gap="4px"
            bg="transparent"
            border={0}
            p="6px"
            color={isActive ? "#FFFFFF" : "#9A9A9A"}
            onClick={() => onChange(tab.key)}
            style={{ cursor: "pointer" }}
          >
            {/* 활성 dot 인디케이터 */}
            <Box
              position="relative"
              display="flex"
              alignItems="center"
              justifyContent="center"
              height="28px"
            >
              {isActive && (
                <Box
                  position="absolute"
                  top="0"
                  left="50%"
                  style={{
                    transform: "translateX(-50%) translateX(-1px)",
                    width: "4px",
                    height: "4px",
                    borderRadius: "50%",
                    backgroundColor: "#16887f",
                  }}
                />
              )}
              <Text fontSize="20px" lineHeight="1" mt="6px">
                {tab.icon}
              </Text>
            </Box>
            <Text fontSize="12px" fontWeight={isActive ? 600 : 400}>
              {tab.label}
            </Text>
          </Flex>
        );
      })}
    </Box>
  );
}
