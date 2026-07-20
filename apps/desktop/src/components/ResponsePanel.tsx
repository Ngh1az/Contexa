import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Check, Copy } from "@phosphor-icons/react";
import type { OverlayPhase } from "../lib/overlayState";

// docs/12 §7.3 (markdown/code rendering), §4.5 (loading skeleton, not a
// generic spinner). No chat-bubble container — response flows as plain text
// (design direction: minimal, Claude.ai/Codex-CLI style).
export function ResponsePanel({
  phase,
  response,
  error,
}: {
  phase: OverlayPhase;
  response: string;
  error: string | null;
}) {
  if (error) {
    return <p className="px-4 py-3 text-sm text-error">{error}</p>;
  }

  if (phase === "processing" && response.length === 0) {
    return (
      <div className="flex flex-col gap-2 px-4 py-3" aria-label="Loading response">
        <div className="h-3 w-3/4 animate-pulse rounded bg-bg-secondary" />
        <div className="h-3 w-1/2 animate-pulse rounded bg-bg-secondary" />
      </div>
    );
  }

  if (response.length === 0) {
    return null;
  }

  return (
    <div className="group relative px-4 py-3">
      <CopyButton text={response} />
      <div
        className="prose-invert max-w-none text-sm leading-relaxed text-text-primary
          [&_code]:font-mono [&_code]:text-[13px] [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-bg-secondary [&_pre]:p-3
          [&_a]:text-accent [&_a:hover]:text-accent-hover"
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
          {response}
        </ReactMarkdown>
      </div>
    </div>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <button
      type="button"
      onClick={copy}
      title="Copy response"
      className="absolute right-2 top-2 rounded-md p-1.5 text-text-secondary opacity-0 transition-opacity
        hover:text-text-primary focus-visible:opacity-100 focus-visible:outline focus-visible:outline-2
        focus-visible:outline-accent group-hover:opacity-100"
    >
      {copied ? <Check size={14} weight="bold" /> : <Copy size={14} />}
    </button>
  );
}
