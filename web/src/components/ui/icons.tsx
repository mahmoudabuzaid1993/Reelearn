import type { SVGProps } from "react";

const base = { width: 16, height: 16, viewBox: "0 0 16 16", fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" } as const;

/** Direction-aware: flips under [dir=rtl]. */
export function ArrowEnd(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} className={`flip-rtl ${props.className ?? ""}`} aria-hidden {...props}>
      <path d="M3 8h10M9 4l4 4-4 4" />
    </svg>
  );
}
export function ArrowStart(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} className={`flip-rtl ${props.className ?? ""}`} aria-hidden {...props}>
      <path d="M13 8H3M7 4L3 8l4 4" />
    </svg>
  );
}
export function Check(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} aria-hidden {...props}>
      <path d="M3 8.5l3 3 7-7" />
    </svg>
  );
}
export function Plus(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} aria-hidden {...props}>
      <path d="M8 3v10M3 8h10" />
    </svg>
  );
}
export function Dot({ className }: { className?: string }) {
  return <span aria-hidden className={`inline-block size-2 rounded-full ${className ?? "bg-slate"}`} />;
}
