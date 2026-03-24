interface Props {
  text: string;
  startMs: number;
  isFinal: boolean;
}

function formatTime(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

export default function TranscriptLine({ text, startMs, isFinal }: Props) {
  return (
    <div className={`flex gap-3 py-1.5 px-2 rounded ${isFinal ? "" : "bg-gray-800/50"}`}>
      <span className="text-xs text-gray-500 font-mono mt-0.5 shrink-0">
        {formatTime(startMs)}
      </span>
      <span className={`text-sm ${isFinal ? "text-gray-200" : "text-gray-400 animate-pulse"}`}>
        {text}
      </span>
    </div>
  );
}
