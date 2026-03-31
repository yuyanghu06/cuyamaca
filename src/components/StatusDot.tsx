interface StatusDotProps {
  status: "green" | "amber" | "red";
}

export default function StatusDot({ status }: StatusDotProps) {
  return <span className={`status-dot ${status}`} />;
}
