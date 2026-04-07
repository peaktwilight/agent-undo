import { motion } from "framer-motion";
import { Github } from "lucide-react";

const links = [
  { label: "Features", href: "#features" },
  { label: "Install", href: "#install" },
  { label: "How it works", href: "#how" },
  { label: "Why", href: "#why" },
];

export default function DockNav() {
  return (
    <motion.header
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, ease: [0.2, 0.7, 0.2, 1], delay: 0.05 }}
      className="fixed top-3 left-1/2 z-50 max-w-[calc(100vw-24px)] -translate-x-1/2"
      style={{ fontFamily: "Outfit, sans-serif" }}
    >
      <div className="flex w-[calc(100vw-24px)] items-center justify-between gap-0.5 rounded-xl border border-white/10 bg-[#0a0a0a]/80 px-2 py-1.5 shadow-2xl shadow-black/30 backdrop-blur-2xl sm:w-auto sm:justify-start sm:px-1.5">
        <a
          href="/"
          className="flex h-9 items-center gap-1.5 overflow-hidden rounded-lg px-2.5 transition-colors hover:bg-white/[0.04]"
        >
          <span className="inline-block h-2 w-2 rounded-sm bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.8)]" />
          <span className="whitespace-nowrap text-[13px] font-bold leading-none tracking-tight text-white">
            agent-undo
          </span>
        </a>

        {links.map((l) => (
          <a
            key={l.href}
            href={l.href}
            className="hidden h-9 items-center rounded-lg px-2.5 transition-colors hover:bg-white/[0.04] sm:flex"
          >
            <span className="whitespace-nowrap text-[11px] font-medium text-white/80">
              {l.label}
            </span>
          </a>
        ))}

        <div className="flex items-center gap-1 sm:ml-0.5">
          <a
            href="https://github.com/peaktwilight/agent-undo"
            target="_blank"
            rel="noopener noreferrer"
            className="flex h-9 items-center gap-1.5 rounded-lg border border-white/15 px-3 text-white/70 transition-colors hover:border-cyan-400/60 hover:text-white"
          >
            <Github className="h-3.5 w-3.5" />
            <span className="text-[11px] font-medium">GitHub</span>
          </a>
        </div>
      </div>
    </motion.header>
  );
}
