"use client";

import { PorscheDesignSystemProvider as PdsProvider } from "@porsche-design-system/components-react/ssr";

export function PorscheDesignSystemProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  return <PdsProvider>{children}</PdsProvider>;
}
