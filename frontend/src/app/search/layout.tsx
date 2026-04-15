"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { PWordmark } from "@porsche-design-system/components-react/ssr";

const NAV_SECTIONS = [
  {
    items: [
      { href: "/search", label: "Signal Search" },
      { href: "/search/imports", label: "Manage Imports" },
    ],
  },
];

export default function SearchLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const pathname = usePathname();

  return (
    <div className="flex min-h-screen">
      {/* ── Sidebar ─────────────────────────────────────────────── */}
      <aside className="w-60 shrink-0 bg-[#f7f7f7] border-r border-gray-200 flex flex-col">
        {/* Brand header */}
        <Link
          href="/"
          className="block px-6 py-5 border-b border-gray-200 hover:bg-gray-100 transition-colors"
        >
          <div className="text-[11px] font-semibold tracking-[0.15em] text-gray-800 uppercase mb-2">
            K-Matrix Toolkit
          </div>
          <PWordmark size="small" />
        </Link>

        {/* Nav items */}
        <nav className="flex-1 py-3">
          {NAV_SECTIONS.map((section, si) => (
            <div key={si}>
              {section.items.map((item) => {
                const active = pathname === item.href;
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={`block px-6 py-2.5 text-[13px] transition-colors border-l-2 ${
                      active
                        ? "border-black bg-white font-semibold text-black"
                        : "border-transparent text-gray-600 hover:bg-gray-100 hover:text-black"
                    }`}
                  >
                    {item.label}
                  </Link>
                );
              })}
            </div>
          ))}
        </nav>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200">
          <p className="text-[10px] text-gray-400">
            Porsche Design System
          </p>
          <p className="text-[10px] text-gray-400 mt-0.5">
            Rust + Next.js
          </p>
        </div>
      </aside>

      {/* ── Main content ────────────────────────────────────────── */}
      <main className="flex-1 overflow-auto bg-white">{children}</main>
    </div>
  );
}
