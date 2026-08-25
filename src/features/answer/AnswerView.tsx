import { useState } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import { Highlight, themes, type Language } from "prism-react-renderer";
import { useTranslation } from "react-i18next";
import "katex/dist/katex.min.css";

function CodeBlock({ code, language }: { code: string; language: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  async function copyCode() {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    globalThis.setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="answer-code-block">
      <div className="answer-code-header">
        <span>{language || "text"}</span>
        <button onClick={() => void copyCode()}>{copied ? t("actions.copied") : t("actions.copy")}</button>
      </div>
      <Highlight theme={themes.vsDark} code={code.replace(/\n$/, "")} language={(language || "text") as Language}>
        {({ className, style, tokens, getLineProps, getTokenProps }) => (
          <pre className={className} style={style}>
            {tokens.map((line, lineIndex) => (
              <div key={lineIndex} {...getLineProps({ line })}>
                <span className="answer-line-number">{lineIndex + 1}</span>
                {line.map((token, tokenIndex) => (
                  <span key={tokenIndex} {...getTokenProps({ token })} />
                ))}
              </div>
            ))}
          </pre>
        )}
      </Highlight>
    </div>
  );
}

const components: Components = {
  code({ className, children, ...props }) {
    const match = /language-([\w-]+)/.exec(className || "");
    const code = String(children);
    if (match || code.includes("\n")) {
      return <CodeBlock code={code} language={match?.[1] || "text"} />;
    }
    return <code className={className} {...props}>{children}</code>;
  },
  a({ children, ...props }) {
    return <a {...props} target="_blank" rel="noreferrer noopener">{children}</a>;
  },
};

export function AnswerView({ content }: { content: string }) {
  const { t } = useTranslation();
  return (
    <article className="answer-markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeKatex]}
        components={components}
        skipHtml
      >
        {content || t("answer.waiting")}
      </ReactMarkdown>
    </article>
  );
}
