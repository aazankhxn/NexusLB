import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 5,
};

export const metadata: Metadata = {
  title: "NexusLB — The Sub-Millisecond Layer 7 Reverse Proxy & Load Balancer",
  description:
    "Engineered in Safe Rust with Tokio async I/O and zero-allocation HTTP streaming. Outperforms NGINX by 1.68x throughput and 3.2x lower P99 tail latency.",
  keywords: [
    "Load Balancer",
    "Reverse Proxy",
    "Rust",
    "Tokio",
    "Zero Allocation",
    "NGINX Alternative",
    "High Performance",
    "Low Latency",
  ],
  authors: [{ name: "NexusLB Team" }],
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/nexuslb.png", type: "image/png" },
    ],
    apple: [
      { url: "/apple-icon.png", sizes: "180x180", type: "image/png" },
    ],
  },
};

import { PrivacyBanner } from "@/components/PrivacyBanner";

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable}`}>
      <head>
        <link rel="icon" href="/favicon.ico" sizes="any" />
        <link rel="icon" href="/nexuslb.png" type="image/png" />
        <link rel="apple-touch-icon" href="/apple-icon.png" />
      </head>
      <body>
        {children}
        <PrivacyBanner />
      </body>
    </html>
  );
}
