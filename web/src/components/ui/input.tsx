import { forwardRef, type InputHTMLAttributes, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

const base =
  "w-full rounded-[10px] border bg-surface px-3 text-sm text-ink placeholder:text-slate/70 disabled:bg-surface-muted disabled:text-slate aria-[invalid=true]:border-ink aria-[invalid=true]:border-2";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function Input({ className, dir = "auto", ...rest }, ref) {
  return <input ref={ref} dir={dir} className={cn(base, "h-10 border-line", className)} {...rest} />;
});

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(function Textarea({ className, dir = "auto", ...rest }, ref) {
  return <textarea ref={ref} dir={dir} className={cn(base, "min-h-24 border-line py-2 leading-relaxed", className)} {...rest} />;
});

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(function Select({ className, children, ...rest }, ref) {
  return (
    <select ref={ref} className={cn(base, "h-10 border-line pe-8 appearance-none bg-no-repeat", className)} style={{ backgroundImage: "url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'><path d='M2 4l4 4 4-4' fill='none' stroke='%235B6B78' stroke-width='1.5'/></svg>\")", backgroundPosition: "right 0.75rem center" }} {...rest}>
      {children}
    </select>
  );
});

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  label: React.ReactNode;
  description?: React.ReactNode;
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(function Checkbox({ label, description, className, ...rest }, ref) {
  return (
    <label className={cn("flex cursor-pointer items-start gap-2.5 text-sm text-ink", className)}>
      <input ref={ref} type="checkbox" className="mt-0.5 size-4 shrink-0 accent-ink" {...rest} />
      <span className="flex flex-col">
        <span>{label}</span>
        {description ? <span className="text-xs text-slate">{description}</span> : null}
      </span>
    </label>
  );
});

export interface SwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: React.ReactNode;
  description?: React.ReactNode;
  disabled?: boolean;
  name?: string;
}

export function Switch({ checked, onChange, label, description, disabled, name }: SwitchProps) {
  return (
    <div className="flex items-start justify-between gap-4">
      <span className="flex flex-col text-sm">
        <span className="text-ink">{label}</span>
        {description ? <span className="text-xs text-slate">{description}</span> : null}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={typeof label === "string" ? label : undefined}
        name={name}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn("relative h-6 w-11 shrink-0 rounded-full border transition-colors disabled:opacity-50", checked ? "border-ink bg-ink" : "border-line bg-surface-muted")}
      >
        <span aria-hidden className={cn("absolute top-0.5 size-4.5 rounded-full bg-white shadow transition-[inset-inline-start]", checked ? "start-[calc(100%-1.25rem)]" : "start-0.5")} />
      </button>
    </div>
  );
}
