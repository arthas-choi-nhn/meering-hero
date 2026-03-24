import { useCallback, useEffect, useRef, useState } from "react";

interface Props {
  onResize: (delta: number) => void;
}

export default function ResizeHandle({ onResize }: Props) {
  const [dragging, setDragging] = useState(false);
  const lastX = useRef(0);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setDragging(true);
    lastX.current = e.clientX;
  }, []);

  useEffect(() => {
    if (!dragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const delta = lastX.current - e.clientX;
      lastX.current = e.clientX;
      onResize(delta);
    };

    const handleMouseUp = () => setDragging(false);

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging, onResize]);

  return (
    <div
      onMouseDown={handleMouseDown}
      className={`w-1 cursor-col-resize hover:bg-blue-500/50 transition-colors shrink-0 ${
        dragging ? "bg-blue-500/70" : "bg-gray-700"
      }`}
    />
  );
}
