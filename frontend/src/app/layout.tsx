import type { Metadata } from "next";
import "./globals.css";
import { PorscheDesignSystemProvider } from "./pds-provider";

export const metadata: Metadata = {
  title: "K-Matrix Toolkit",
  description: "Search and explore automotive K-Matrix files",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="h-full">
      <head />
      <body className="min-h-screen flex flex-col bg-white">
        <PorscheDesignSystemProvider>{children}</PorscheDesignSystemProvider>
      </body>
    </html>
  );
}
