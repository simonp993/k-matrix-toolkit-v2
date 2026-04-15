"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    setMounted(true);

    // Resume video when returning via browser back (bfcache)
    const onPageShow = (e: PageTransitionEvent) => {
      if (e.persisted && videoRef.current) {
        videoRef.current.play().catch(() => {});
      }
    };
    window.addEventListener("pageshow", onPageShow);
    return () => window.removeEventListener("pageshow", onPageShow);
  }, []);

  return (
    <div className="relative grid min-h-screen grid-rows-[minmax(0,1fr)_auto] overflow-hidden">
      {/* ── Hero Video Background (same approach as PDS website) ───── */}
      <video
        ref={videoRef}
        className="row-start-1 col-start-1 w-full h-screen object-cover"
        poster="/assets/hero-light.jpg"
        loop
        muted
        autoPlay
        playsInline
        disablePictureInPicture
        controlsList="nodownload nofullscreen noplaybackrate noremoteplayback"
        aria-hidden="true"
        tabIndex={-1}
      >
        <source src="/assets/hero-light.mp4" type="video/mp4" />
        <source src="/assets/hero-light.webm" type="video/webm" />
      </video>

      {/* ── Content overlay ───────────────────────────────────────── */}
      <div
        className="row-start-1 col-start-1 z-20 flex flex-col justify-center px-16 md:px-24 lg:px-32 py-24 pointer-events-none"
      >
        <div
          style={{
            opacity: mounted ? 1 : 0,
            transform: mounted ? "translateY(0)" : "translateY(40px)",
            transition: "opacity 1s ease-out 300ms, transform 1.2s ease-out 300ms",
          }}
        >
          <h1
            className="leading-[1.1] tracking-[-0.02em] mb-8 max-w-3xl"
            style={{
              fontSize: "clamp(3rem, 7vw, 5.5rem)",
              fontWeight: 400,
              fontFamily: "'Porsche Next', Arial, sans-serif",
            }}
          >
            Welcome to the
            <br />
            Porsche K-Matrix
            <br />
            Search Tool
          </h1>

          <p
            className="text-lg text-gray-600 mb-12 max-w-xl leading-relaxed"
            style={{ fontFamily: "'Porsche Next', Arial, sans-serif" }}
          >
            Import, search, and explore automotive K-Matrix signal databases
            across CAN, LIN, and Ethernet. Making the still existence of the K-Matrix a little more bearable.
          </p>

          <Link
            href="/search"
            className="pointer-events-auto inline-flex items-center gap-2 bg-[#010205] text-white px-8 py-3.5 
                       rounded-sm text-sm font-medium tracking-wide
                       hover:bg-[#1a1d25] transition-colors"
            style={{ fontFamily: "'Porsche Next', Arial, sans-serif" }}
          >
            Open Search Tool
          </Link>
        </div>
      </div>

      {/* ── Bottom card — links to GitHub repo ────────────────────── */}
      <div className="row-start-1 col-start-1 z-20 pointer-events-none">
        <a
          href="https://github.com/simonp993/k-matrix-toolkit-v2"
          target="_blank"
          rel="noopener noreferrer"
          className="pointer-events-auto absolute bg-white/90 backdrop-blur-md rounded-lg shadow-lg 
                     flex items-center justify-between gap-4 max-w-lg px-6 py-5
                     hover:bg-white hover:shadow-xl transition-all"
          style={{
            bottom: "5%",
            left: "5%",
            opacity: mounted ? 1 : 0,
            transform: mounted ? "translateY(0)" : "translateY(24px)",
            transition:
              "opacity 1.2s ease-out 800ms, transform 1.2s ease-out 800ms, background-color 0.2s, box-shadow 0.2s",
          }}
        >
          <div className="flex flex-col">
            <p
              className="font-semibold text-sm text-gray-900"
              style={{ fontFamily: "'Porsche Next', Arial, sans-serif" }}
            >
              K-Matrix Toolkit v2
            </p>
            <p
              className="text-xs text-gray-500 mt-0.5"
              style={{ fontFamily: "'Porsche Next', Arial, sans-serif" }}
            >
              Rust + Next.js &middot; Porsche Design System
            </p>
          </div>
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-gray-400"
          >
            <path d="M5 12h14M12 5l7 7-7 7" />
          </svg>
        </a>
      </div>
    </div>
  );
}
