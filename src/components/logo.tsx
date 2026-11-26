import { useId } from "react";
import { cn } from "@/lib/utils";

interface LogoProps {
  className?: string;
  compact?: boolean;
  size?: "sm" | "md" | "lg";
}

const sizes = {
  sm: "h-8 w-8",
  md: "h-9 w-9",
  lg: "h-14 w-14",
};

export function Logo({ className, compact = false, size = "md" }: LogoProps) {
  const gradientId = useId().replace(/:/g, "");

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <svg viewBox="0 0 64 64" className={sizes[size]} aria-hidden="true">
        <defs>
          <linearGradient
            id={gradientId}
            x1="6"
            y1="4"
            x2="58"
            y2="60"
            gradientUnits="userSpaceOnUse"
          >
            <stop stopColor="#818cf8" />
            <stop offset="0.55" stopColor="#6366f1" />
            <stop offset="1" stopColor="#22d3ee" />
          </linearGradient>
        </defs>
        <rect
          x="2"
          y="2"
          width="60"
          height="60"
          rx="17"
          fill={`url(#${gradientId})`}
        />
        <rect
          x="2.75"
          y="2.75"
          width="58.5"
          height="58.5"
          rx="16.25"
          fill="none"
          stroke="white"
          strokeOpacity="0.18"
          strokeWidth="1.5"
        />
        <path d="M26.5 20.5 45 32 26.5 43.5Z" fill="white" />
        <circle cx="19" cy="32" r="2.6" fill="white" fillOpacity="0.7" />
      </svg>

      {!compact ? (
        <div className="leading-none">
          <div className="text-[15px] font-semibold tracking-tight">
            Glasswing
          </div>
          <div className="mt-1 text-[11px] text-muted-foreground">
            PS5 Remote Play
          </div>
        </div>
      ) : null}
    </div>
  );
}
