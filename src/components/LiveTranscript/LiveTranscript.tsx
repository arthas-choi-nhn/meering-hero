import { useEffect, useRef } from "react";
import type { TranscriptLine as TranscriptLineType } from "../../hooks/useTranscript";
import TranscriptLine from "./TranscriptLine";

interface Props {
  lines: TranscriptLineType[];
  isRecording: boolean;
}

export default function LiveTranscript({ lines, isRecording }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines]);

  if (lines.length === 0 && !isRecording) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500">
        <p>녹음을 시작하면 전사 내용이 여기에 표시됩니다</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-1">
      {lines.map((line, i) => (
        <TranscriptLine
          key={i}
          text={line.text}
          startMs={line.startMs}
          isFinal={line.isFinal}
        />
      ))}
      {isRecording && lines.length === 0 && (
        <div className="text-gray-500 text-sm animate-pulse">
          음성을 인식하고 있습니다...
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
