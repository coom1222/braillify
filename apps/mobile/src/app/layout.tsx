import { globalCss } from '@devup-ui/react'
import type { Metadata, Viewport } from 'next'

export const metadata: Metadata = {
  title: 'Braillify',
  description: '한국어 텍스트를 한국 점자로 변환하는 점역기',
}

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 1,
  userScalable: false,
  viewportFit: 'cover',
}

// 기존 src/global.css 를 devup-ui globalCss 로 이식 (#root 의존 제거)
globalCss({
  '*, *::before, *::after': {
    boxSizing: 'border-box',
  },
  'html, body': {
    height: '100%',
  },
  body: {
    margin: 0,
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", Pretendard, "Noto Sans KR", "Segoe UI", Roboto, sans-serif',
    background: '#f2f2f2',
    color: '#1a1a1a',
  },
  button: {
    fontFamily: 'inherit',
    cursor: 'pointer',
  },
  'input, textarea': {
    fontFamily: 'inherit',
  },
})

// devup 타입이 노출하지 않는 vendor 속성은 raw globalCss 로 정의 (모바일 UX)
globalCss`body{-webkit-font-smoothing:antialiased;-webkit-tap-highlight-color:transparent}`

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="ko">
      <body>{children}</body>
    </html>
  )
}
