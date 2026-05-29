import { useState } from "react";
import { Box, Flex, Text } from "@devup-ui/react";
import { Textarea } from "@devup-ui/components";
import { translate, translateReverse } from "../lib/translate";
import { pushHistory, type TranslateMode } from "../lib/history";
import { copyText } from "../lib/clipboard";
import { masksToString } from "../lib/braille";
import { BrailleInput } from "../components/BrailleInput";

type Direction = "forward" | "reverse";

type ResultState =
  | { kind: "idle" }
  | { kind: "ok"; output: string }
  | { kind: "error"; message: string };

export function TranslatorPage() {
  const [text, setText] = useState("");
  const [cells, setCells] = useState<number[]>([]); // 역점역 입력용 점자 셀 배열
  const [direction, setDirection] = useState<Direction>("forward");
  const [mode, setMode] = useState<TranslateMode>("general");
  const [result, setResult] = useState<ResultState>({ kind: "idle" });
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">(
    "idle",
  );

  // 역점역 모드에서 실제 점역할 문자열
  const reverseBraille = masksToString(cells);
  // 번역 버튼 활성화 조건
  const canTranslate =
    direction === "forward" ? !!text.trim() : cells.length > 0;

  function handleTranslate() {
    if (direction === "forward") {
      const r = translate(text, mode);
      if (r.ok) {
        setResult({ kind: "ok", output: r.braille });
        pushHistory({ source: text, braille: r.braille, mode });
      } else {
        setResult({ kind: "error", message: r.error });
      }
    } else {
      const r = translateReverse(reverseBraille);
      if (r.ok) {
        setResult({ kind: "ok", output: r.korean });
      } else {
        setResult({ kind: "error", message: r.error });
      }
    }
    setCopyState("idle");
  }

  async function handleCopy() {
    if (result.kind !== "ok") return;
    try {
      await copyText(result.output);
      setCopyState("copied");
      setTimeout(() => setCopyState("idle"), 1500);
    } catch {
      setCopyState("error");
      setTimeout(() => setCopyState("idle"), 1500);
    }
  }

  function handleModeChange(next: TranslateMode) {
    setMode(next);
    setResult({ kind: "idle" });
  }

  function handleDirectionChange(next: Direction) {
    setDirection(next);
    setText("");
    setCells([]);
    setResult({ kind: "idle" });
  }

  return (
    <Box px="20px" py="24px">
      <Text as="h1" fontSize="24px" fontWeight={700} m={0} mb="6px">
        점역기
      </Text>
      <Text as="p" m={0} mb="16px" color="$textMuted" fontSize="14px">
        {direction === "forward"
          ? "한글 텍스트를 입력하면 2024 개정 한국 점자 규정에 따라 점역합니다."
          : "점을 눌러 점자를 조합하고 역점역합니다."}
      </Text>

      {/* Direction toggle */}
      <Flex
        display="inline-flex"
        bg="$surface"
        border="1px solid"
        borderColor="$border"
        borderRadius="999px"
        p="4px"
        mb="12px"
        gap="4px"
      >
        {(["forward", "reverse"] as const).map((d) => {
          const active = direction === d;
          return (
            <Box
              key={d}
              as="button"
              type="button"
              role="tab"
              aria-selected={active}
              border={0}
              borderRadius="999px"
              px="18px"
              py="6px"
              fontSize="13px"
              fontWeight={active ? 600 : 500}
              bg={active ? "$primary" : "transparent"}
              color={active ? "$primaryText" : "$textMuted"}
              onClick={() => handleDirectionChange(d)}
            >
              {d === "forward" ? "정점역" : "역점역"}
            </Box>
          );
        })}
      </Flex>

      {/* Mode toggle — 정점역일 때만 표시 */}
      {direction === "forward" && (
        <Flex
          display="inline-flex"
          bg="$surface"
          border="1px solid"
          borderColor="$border"
          borderRadius="999px"
          p="4px"
          mb="16px"
          ml="8px"
          gap="4px"
        >
          {(["general", "math"] as const).map((m) => {
            const active = mode === m;
            return (
              <Box
                key={m}
                as="button"
                type="button"
                role="tab"
                aria-selected={active}
                border={0}
                borderRadius="999px"
                px="18px"
                py="6px"
                fontSize="13px"
                fontWeight={active ? 600 : 500}
                bg={active ? "$primary" : "transparent"}
                color={active ? "$primaryText" : "$textMuted"}
                onClick={() => handleModeChange(m)}
              >
                {m === "general" ? "일반" : "수학"}
              </Box>
            );
          })}
        </Flex>
      )}

      {/* Input card */}
      <Box
        bg="$surface"
        borderRadius="12px"
        border="1px solid"
        borderColor="$border"
        px="16px"
        pt="14px"
        pb="12px"
      >
        <Flex
          justifyContent="space-between"
          alignItems="baseline"
          pb="8px"
          borderBottom="1px solid"
          borderColor="$border"
        >
          <Text fontSize="14px" fontWeight={600}>
            {direction === "forward" ? "입력 텍스트" : "점자 입력"}
          </Text>
          <Text fontSize="12px" color="$textSubtle">
            {direction === "forward" ? `${text.length}자` : `${cells.length}셀`}
          </Text>
        </Flex>

        <Box py="8px">
          {direction === "reverse" ? (
            <BrailleInput cells={cells} onChange={setCells} />
          ) : (
            <Textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              rows={4}
              placeholder={
                mode === "math"
                  ? "LaTeX 수식을 $...$로 감싸 입력하세요. 예: $\\frac{3}{4}$"
                  : "점역할 텍스트를 입력하세요..."
              }
            />
          )}
        </Box>

        <Flex
          justifyContent="space-between"
          alignItems="center"
          pt="8px"
          borderTop="1px solid"
          borderColor="$border"
        >
          <Text fontSize="12px" color="$textSubtle">
            {direction === "reverse"
              ? "역점역 모드"
              : mode === "math"
                ? "수학 모드 (LaTeX)"
                : "일반 모드"}
          </Text>
          <Box
            as="button"
            type="button"
            bg="$primary"
            color="$primaryText"
            border={0}
            borderRadius="8px"
            px="18px"
            py="10px"
            fontSize="13px"
            fontWeight={600}
            opacity={canTranslate ? 1 : 0.4}
            cursor={canTranslate ? "pointer" : "not-allowed"}
            disabled={!canTranslate}
            onClick={handleTranslate}
          >
            {direction === "forward" ? "점역하기" : "역점역하기"}
          </Box>
        </Flex>
      </Box>

      {/* Result */}
      <Flex
        mt="28px"
        minH="120px"
        alignItems="center"
        justifyContent="center"
        textAlign="center"
        aria-live="polite"
      >
        {result.kind === "idle" && (
          <Text color="$textMuted" fontSize="14px">
            {direction === "forward"
              ? "텍스트를 입력하고 점역을 시작해보세요"
              : "점자를 입력하고 역점역을 시작해보세요"}
          </Text>
        )}
        {result.kind === "ok" && (
          <Flex flexDirection="column" alignItems="center" gap="16px" w="100%">
            <Box
              fontSize={direction === "forward" ? "28px" : "20px"}
              lineHeight={1.6}
              wordBreak="break-all"
              letterSpacing={direction === "forward" ? "2px" : "0px"}
              color="$text"
            >
              {result.output}
            </Box>
            <Box
              as="button"
              type="button"
              bg="$surface"
              color="$text"
              border="1px solid"
              borderColor="$border"
              borderRadius="8px"
              px="18px"
              py="8px"
              fontSize="13px"
              fontWeight={500}
              onClick={handleCopy}
            >
              {copyState === "copied"
                ? "복사됨"
                : copyState === "error"
                  ? "복사 실패"
                  : "복사"}
            </Box>
          </Flex>
        )}
        {result.kind === "error" && (
          <Text
            as="p"
            color="$danger"
            fontSize="14px"
            bg="$dangerSoft"
            border="1px solid"
            borderColor="$dangerSoftBorder"
            borderRadius="8px"
            px="16px"
            py="12px"
            m={0}
            w="100%"
          >
            {result.message}
          </Text>
        )}
      </Flex>
    </Box>
  );
}
