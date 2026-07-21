import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Check, Copy } from "@phosphor-icons/react";
import type { Message, OverlayPhase } from "../lib/overlayState";

// docs/12 §7.3 (markdown/code rendering) + Claude.ai-style bubbles: user
// messages get a bubble (bg-tertiary), assistant responses stay plain text
// (no bubble) — same minimal direction ResponsePanel used, extended to a
// scrollable multi-turn list.
export function MessageList({
  phase,
  messages,
  error,
}: {
  phase: OverlayPhase;
  messages: Message[];
  error: string | null;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll) scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages, autoScroll]);

  const onScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    setAutoScroll(atBottom);
  };

  if (messages.length === 0 && !error) {
    return <div className="flex-1" />;
  }

  const lastMessage = messages[messages.length - 1];

  return (
    <div ref={scrollRef} onScroll={onScroll} className="flex-1 overflow-y-auto px-4 py-3">
      {messages.map((message) => (
        <MessageBubble key={message.id} message={message} />
      ))}
      {phase === "processing" && lastMessage?.content.length === 0 && (
        <div className="flex flex-col gap-2 py-1" aria-label="Loading response">
          <div className="h-3 w-3/4 animate-pulse rounded bg-bg-secondary" />
          <div className="h-3 w-1/2 animate-pulse rounded bg-bg-secondary" />
        </div>
      )}
      {error && <p className="pt-2 text-sm text-error">{error}</p>}
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  if (message.role === "user") {
    return (
      <div className="mb-3 flex justify-end">
        <div className="max-w-[75%] rounded-[12px_12px_2px_12px] bg-bg-tertiary px-3 py-1.5 text-sm text-text-primary">
          {message.content}
        </div>
      </div>
    );
  }

  if (message.content.length === 0) return null;

  return (
    <div className="group relative mb-3">
      <CopyButton text={message.content} />
      <div
        className="prose-invert max-w-none text-sm leading-relaxed text-text-primary
          [&_code]:font-mono [&_code]:text-[13px] [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-bg-secondary [&_pre]:p-3
          [&_a]:text-accent [&_a:hover]:text-accent-hover"
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
          {message.content}
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
