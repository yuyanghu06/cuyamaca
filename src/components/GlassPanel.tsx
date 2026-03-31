import { ReactNode } from "react";

interface GlassPanelProps {
  tier?: "subtle" | "standard" | "strong";
  className?: string;
  children: ReactNode;
  style?: React.CSSProperties;
}

export default function GlassPanel({
  tier = "standard",
  className = "",
  children,
  style,
}: GlassPanelProps) {
  return (
    <div className={`glass-${tier} ${className}`} style={style}>
      {children}
    </div>
  );
}
