import { useEffect, useState } from "react";

function GithubIcon(props: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="currentColor"
      className={props.className}
      aria-hidden="true"
    >
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}

const links = [
  { label: "Features", href: "#features" },
  { label: "Install", href: "#install" },
  { label: "How", href: "#how" },
  { label: "Why", href: "#why" },
];

export default function DockNav() {
  // Two-frame mount transition: invisible/up -> visible/0
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    const t = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(t);
  }, []);

  return (
    <header
      className="fixed top-3 left-1/2 z-50 max-w-[calc(100vw-24px)] -translate-x-1/2"
      style={{
        fontFamily: "Outfit, sans-serif",
        opacity: mounted ? 1 : 0,
        transform: `translate(-50%, ${mounted ? "0" : "-12px"})`,
        transition:
          "opacity 600ms cubic-bezier(0.2,0.7,0.2,1), transform 600ms cubic-bezier(0.2,0.7,0.2,1)",
        willChange: "opacity, transform",
      }}
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
            <GithubIcon className="h-3.5 w-3.5" />
            <span className="text-[11px] font-medium">GitHub</span>
          </a>
        </div>
      </div>
    </header>
  );
}
