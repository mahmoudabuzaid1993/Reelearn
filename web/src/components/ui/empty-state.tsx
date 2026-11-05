import type { ReactNode } from "react";

export function EmptyState({ title, description, action }: { title: ReactNode; description?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-[14px] border border-dashed border-line px-6 py-12 text-center">
      <p className="font-display text-[15px] font-semibold text-ink">{title}</p>
      {description ? <p className="max-w-md text-[13px] text-slate">{description}</p> : null}
      {action ? <div className="mt-3">{action}</div> : null}
    </div>
  );
}
