import { useState, useEffect, useCallback, useRef } from "react";
import {
  listDoorayWikis,
  listDoorayWikiPages,
  createDoorayWikiPage,
  updateDoorayWikiPage,
  type DoorayWiki,
  type WikiPage,
  type Summary,
} from "../lib/tauri-commands";

interface Props {
  open: boolean;
  onClose: () => void;
  sessionId: string;
  sessionTitle: string;
  latestSummary: Summary | null;
}

type Mode = "new" | "existing";

interface BreadcrumbItem {
  id: string | null;
  title: string;
}

export default function DoorayExportDialog({
  open,
  onClose,
  sessionId,
  sessionTitle,
  latestSummary,
}: Props) {
  const [mode, setMode] = useState<Mode>("new");
  const [wikis, setWikis] = useState<DoorayWiki[]>([]);
  const [selectedWiki, setSelectedWiki] = useState<string>("");
  const [pages, setPages] = useState<WikiPage[]>([]);
  const [breadcrumb, setBreadcrumb] = useState<BreadcrumbItem[]>([
    { id: null, title: "최상위" },
  ]);
  const [selectedPageId, setSelectedPageId] = useState<string | null>(null);
  const [title, setTitle] = useState(sessionTitle);
  const [wikiSearch, setWikiSearch] = useState("");
  const [wikiDropdownOpen, setWikiDropdownOpen] = useState(false);
  const wikiDropdownRef = useRef<HTMLDivElement>(null);
  const [loading, setLoading] = useState(false);
  const [loadingPages, setLoadingPages] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (wikiDropdownRef.current && !wikiDropdownRef.current.contains(e.target as Node)) {
        setWikiDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setSuccess(null);
    setTitle(sessionTitle);
    setSelectedPageId(null);
    setBreadcrumb([{ id: null, title: "최상위" }]);
    listDoorayWikis()
      .then(setWikis)
      .catch((e) => setError(String(e)));
  }, [open, sessionTitle]);

  const currentParentId = breadcrumb[breadcrumb.length - 1].id;

  const loadPages = useCallback(
    async (wikiId: string, parentId: string | null) => {
      if (!wikiId) return;
      setLoadingPages(true);
      try {
        const result = await listDoorayWikiPages(wikiId, parentId);
        setPages(result);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoadingPages(false);
      }
    },
    [],
  );

  useEffect(() => {
    if (selectedWiki) {
      loadPages(selectedWiki, currentParentId);
    }
  }, [selectedWiki, currentParentId, loadPages]);

  const handleDrillDown = (page: WikiPage) => {
    setBreadcrumb((prev) => [...prev, { id: page.id, title: page.subject }]);
    setSelectedPageId(null);
  };

  const handleBreadcrumbClick = (index: number) => {
    setBreadcrumb((prev) => prev.slice(0, index + 1));
    setSelectedPageId(null);
  };

  const handleExport = async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      if (!selectedWiki) {
        setError("위키를 선택해주세요.");
        return;
      }

      if (mode === "existing") {
        if (!selectedPageId) {
          setError("업데이트할 위키 페이지를 선택해주세요.");
          return;
        }
        await updateDoorayWikiPage(
          sessionId,
          latestSummary?.id ?? null,
          selectedWiki,
          selectedPageId,
        );
        setSuccess("기존 위키 페이지에 내용이 업데이트되었습니다.");
      } else {
        const parentId = selectedPageId ?? currentParentId;
        await createDoorayWikiPage(
          sessionId,
          latestSummary?.id ?? null,
          selectedWiki,
          parentId,
          title || sessionTitle,
        );
        setSuccess("위키 페이지가 생성되었습니다.");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-xl p-6 w-[520px] max-h-[80vh] overflow-y-auto shadow-2xl">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white">
            Dooray Wiki로 내보내기
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white text-xl"
          >
            &times;
          </button>
        </div>

        {/* Mode Selection */}
        <div className="flex gap-2 mb-4">
          <button
            onClick={() => { setMode("new"); setSelectedPageId(null); }}
            className={`flex-1 py-2 text-sm rounded-lg transition-colors ${
              mode === "new"
                ? "bg-blue-600 text-white"
                : "bg-gray-700 text-gray-400 hover:text-white"
            }`}
          >
            새 위키 페이지
          </button>
          <button
            onClick={() => { setMode("existing"); setSelectedPageId(null); }}
            className={`flex-1 py-2 text-sm rounded-lg transition-colors ${
              mode === "existing"
                ? "bg-blue-600 text-white"
                : "bg-gray-700 text-gray-400 hover:text-white"
            }`}
          >
            기존 페이지에 작성
          </button>
        </div>

        {/* Wiki Selection with Search */}
        <div className="mb-4 relative" ref={wikiDropdownRef}>
          <label className="block text-xs text-gray-400 mb-1">업무 선택</label>
          <input
            type="text"
            value={wikiDropdownOpen ? wikiSearch : (wikis.find(w => w.id === selectedWiki)?.name ?? "")}
            onChange={(e) => {
              setWikiSearch(e.target.value);
              if (!wikiDropdownOpen) setWikiDropdownOpen(true);
            }}
            onFocus={() => {
              setWikiDropdownOpen(true);
              setWikiSearch("");
            }}
            placeholder="위키를 검색하세요"
            className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          {wikiDropdownOpen && (
            <div className="absolute z-10 w-full mt-1 bg-gray-700 rounded-lg shadow-xl max-h-56 overflow-y-auto border border-gray-600">
              {wikis
                .filter((w) =>
                  wikiSearch === "" || w.name.toLowerCase().includes(wikiSearch.toLowerCase())
                )
                .map((w) => (
                  <div
                    key={w.id}
                    className={`px-3 py-2 text-sm cursor-pointer transition-colors ${
                      selectedWiki === w.id
                        ? "bg-blue-600/30 text-white"
                        : "text-gray-300 hover:bg-gray-600"
                    }`}
                    onClick={() => {
                      setSelectedWiki(w.id);
                      setWikiSearch("");
                      setWikiDropdownOpen(false);
                      setBreadcrumb([{ id: null, title: "최상위" }]);
                      setSelectedPageId(null);
                    }}
                  >
                    {w.name}
                  </div>
                ))}
              {wikis.filter((w) =>
                wikiSearch === "" || w.name.toLowerCase().includes(wikiSearch.toLowerCase())
              ).length === 0 && (
                <div className="px-3 py-2 text-xs text-gray-500">
                  검색 결과가 없습니다.
                </div>
              )}
            </div>
          )}
        </div>

        {/* Wiki Page Browser */}
        {selectedWiki && (
          <div className="mb-4">
            <label className="block text-xs text-gray-400 mb-1">
              {mode === "existing"
                ? "업데이트할 위키 페이지 선택"
                : "상위 위키 선택 (미선택 시 현재 위치에 생성)"}
            </label>

            {/* Breadcrumb */}
            <div className="flex items-center gap-1 mb-2 text-xs text-gray-500 flex-wrap">
              {breadcrumb.map((item, i) => (
                <span key={i} className="flex items-center gap-1">
                  {i > 0 && <span className="text-gray-600">/</span>}
                  <button
                    onClick={() => handleBreadcrumbClick(i)}
                    className="text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    {item.title}
                  </button>
                </span>
              ))}
            </div>

            {/* Page List */}
            <div className="bg-gray-700/50 rounded-lg max-h-48 overflow-y-auto">
              {loadingPages ? (
                <div className="p-3 text-xs text-gray-400 animate-pulse">
                  로딩 중...
                </div>
              ) : pages.length === 0 ? (
                <div className="p-3 text-xs text-gray-500">
                  {mode === "new"
                    ? "이 위치에 새 페이지가 생성됩니다."
                    : "위키 페이지가 없습니다."}
                </div>
              ) : (
                pages.map((page) => (
                  <div
                    key={page.id}
                    className={`flex items-center justify-between px-3 py-2 text-sm cursor-pointer transition-colors ${
                      selectedPageId === page.id
                        ? "bg-blue-600/30 text-white"
                        : "text-gray-300 hover:bg-gray-600/50"
                    }`}
                    onClick={() => {
                      setSelectedPageId(
                        selectedPageId === page.id ? null : page.id,
                      );
                    }}
                  >
                    <span className="flex items-center gap-2 min-w-0">
                      <span className="truncate">{page.subject}</span>
                    </span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDrillDown(page);
                      }}
                      className="text-xs text-gray-400 hover:text-blue-400 shrink-0 ml-2 px-1.5 py-0.5 rounded bg-gray-600/50"
                    >
                      하위
                    </button>
                  </div>
                ))
              )}
            </div>

            {mode === "new" && selectedPageId && (
              <p className="text-xs text-blue-400 mt-1">
                선택한 페이지 하위에 생성됩니다.
              </p>
            )}
          </div>
        )}

        {/* Title (new mode only) */}
        {mode === "new" && (
          <div className="mb-4">
            <label className="block text-xs text-gray-400 mb-1">
              페이지 제목
            </label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="회의록 제목"
              className="w-full bg-gray-700 text-white rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        )}

        {/* Export Content Preview */}
        <div className="mb-4 bg-gray-700/30 rounded-lg p-3">
          <p className="text-xs text-gray-400 mb-2">내보내기 내용</p>
          <div className="flex flex-wrap gap-2 text-xs">
            <span className="px-2 py-0.5 bg-gray-600 text-gray-300 rounded">회의 정보</span>
            {latestSummary ? (
              <span className="px-2 py-0.5 bg-purple-600/30 text-purple-300 rounded">요약</span>
            ) : (
              <span className="px-2 py-0.5 bg-gray-600/50 text-gray-500 rounded line-through">요약 (없음)</span>
            )}
            <span className="px-2 py-0.5 bg-gray-600 text-gray-300 rounded">노트</span>
            <span className="px-2 py-0.5 bg-gray-600 text-gray-300 rounded">전사본 (접기)</span>
          </div>
        </div>

        {/* Error / Success */}
        {error && <p className="text-xs text-red-400 mb-3">{error}</p>}
        {success && <p className="text-xs text-green-400 mb-3">{success}</p>}

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors"
          >
            {success ? "닫기" : "취소"}
          </button>
          {!success && (
            <button
              onClick={handleExport}
              disabled={loading || !selectedWiki}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm rounded-lg transition-colors"
            >
              {loading
                ? "내보내는 중..."
                : mode === "existing"
                  ? "페이지 업데이트"
                  : "페이지 생성"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
